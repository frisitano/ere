// LLVM pass plugin for the ZisK SDK: lowers small constant-size memcpy, memset and memcmp/bcmp to
// ZisK's inline DMA precompiles, the way ZisK's own compiler does.
//
// Stock LLVM expands such a copy into loads and stores during code generation, before any call to
// the SDK's (DMA-backed) `memcpy` exists. ZisK's transpiler instead fuses the two-instruction
// patterns below, as `ziskos`'s `ziskos_memcpy!`/`ziskos_memcmp!`/`ziskos_memset!` macros emit
// them, into one `dma_xmemcpy`/`dma_xmemcmp`/`dma_xmemset` operation:
//
//   memcpy(dst, src, n)    csrs  0x813, src        ; addi x0, dst, n
//   memcmp(a, b, n)        csrrs res, 0x814, b     ; addi x0, a, n     (res: a[i] - b[i], sign-extended)
//   memset(dst, v, n)      csrsi 0x816, 2          ; addi x0, dst, n ; addi x0, dst, v
//
// The pass runs at the end of the full-LTO optimization pipeline, so the optimizer has already
// removed every copy it could, and code generation follows directly. `link.sh` loads it with
// `ld.lld --load-pass-plugin` when the SDK carries it.

#include "llvm/IR/IRBuilder.h"
#include "llvm/IR/InlineAsm.h"
#include "llvm/IR/InstIterator.h"
#include "llvm/IR/IntrinsicInst.h"
#include "llvm/IR/PassManager.h"
#include "llvm/Passes/PassBuilder.h"
#include "llvm/Plugins/PassPlugin.h"

#include <optional>
#include <string>

using namespace llvm;

namespace {

constexpr unsigned MemcpyCsr = 0x813;
constexpr unsigned MemcmpCsr = 0x814;
constexpr unsigned MemsetCsr = 0x816;

// Shorter copies stay in registers. The upper bound is `addi`'s 12-bit signed immediate.
constexpr uint64_t MinBytes = 16;
constexpr uint64_t MaxBytes = 2047;

std::optional<uint64_t> smallLength(const Value *Length) {
  const auto *C = dyn_cast<ConstantInt>(Length);
  if (!C || C->getBitWidth() > 64)
    return std::nullopt;
  uint64_t N = C->getZExtValue();
  if (N < MinBytes || N > MaxBytes)
    return std::nullopt;
  return N;
}

std::string hex(unsigned V) {
  return "0x" + utohexstr(V, /*LowerCase=*/true);
}

struct ZiskDmaPass : PassInfoMixin<ZiskDmaPass> {
  PreservedAnalyses run(Module &M, ModuleAnalysisManager &) {
    LLVMContext &Ctx = M.getContext();
    Type *Ptr = PointerType::get(Ctx, 0);
    Type *Void = Type::getVoidTy(Ctx);
    Type *I64 = Type::getInt64Ty(Ctx);

    SmallVector<Instruction *, 64> Lowered;
    for (Function &F : M) {
      for (Instruction &I : instructions(F)) {
        IRBuilder<> B(&I);
        if (auto *Copy = dyn_cast<MemCpyInst>(&I)) {
          auto N = smallLength(Copy->getLength());
          if (!N || Copy->isVolatile() || Copy->getDestAddressSpace() != 0 ||
              Copy->getSourceAddressSpace() != 0)
            continue;
          auto *Asm = InlineAsm::get(
              FunctionType::get(Void, {Ptr, Ptr}, false),
              "csrs " + hex(MemcpyCsr) + ", $1\n\taddi x0, $0, " + std::to_string(*N),
              "r,r,~{memory}", /*hasSideEffects=*/true);
          B.CreateCall(Asm, {Copy->getRawDest(), Copy->getRawSource()});
          Lowered.push_back(&I);
        } else if (auto *Set = dyn_cast<MemSetInst>(&I)) {
          auto N = smallLength(Set->getLength());
          const auto *Byte = dyn_cast<ConstantInt>(Set->getValue());
          if (!N || !Byte || Set->isVolatile() || Set->getDestAddressSpace() != 0)
            continue;
          auto *Asm = InlineAsm::get(
              FunctionType::get(Void, {Ptr}, false),
              "csrsi " + hex(MemsetCsr) + ", 2\n\taddi x0, $0, " + std::to_string(*N) +
                  "\n\taddi x0, $0, " + std::to_string(Byte->getZExtValue() & 0xff),
              "r,~{memory}", /*hasSideEffects=*/true);
          B.CreateCall(Asm, {Set->getRawDest()});
          Lowered.push_back(&I);
        } else if (auto *Call = dyn_cast<CallInst>(&I)) {
          const Function *Callee = Call->getCalledFunction();
          if (!Callee || Call->arg_size() != 3 || !Call->getType()->isIntegerTy() ||
              (Callee->getName() != "memcmp" && Callee->getName() != "bcmp"))
            continue;
          auto N = smallLength(Call->getArgOperand(2));
          if (!N)
            continue;
          auto *Asm = InlineAsm::get(
              FunctionType::get(I64, {Ptr, Ptr}, false),
              "csrrs $0, " + hex(MemcmpCsr) + ", $2\n\taddi x0, $1, " + std::to_string(*N),
              "=r,r,r,~{memory}", /*hasSideEffects=*/true);
          Value *Diff = B.CreateCall(Asm, {Call->getArgOperand(0), Call->getArgOperand(1)});
          Call->replaceAllUsesWith(B.CreateTrunc(Diff, Call->getType()));
          Lowered.push_back(&I);
        }
      }
    }
    for (Instruction *I : Lowered)
      I->eraseFromParent();
    return Lowered.empty() ? PreservedAnalyses::all() : PreservedAnalyses::none();
  }
};

} // namespace

extern "C" LLVM_ATTRIBUTE_WEAK PassPluginLibraryInfo llvmGetPassPluginInfo() {
  return {LLVM_PLUGIN_API_VERSION, "zisk-dma", "0.1", [](PassBuilder &PB) {
            PB.registerFullLinkTimeOptimizationLastEPCallback(
                [](ModulePassManager &MPM, OptimizationLevel) { MPM.addPass(ZiskDmaPass()); });
          }};
}
