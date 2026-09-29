//! SP1 SDK library. Everything comes from SP1's `libzkevm`, built for a target with
//! `os = "zkvm"` and `vendor = "succinct"` so `sp1-zkvm` and SP1's patched crypto crates select
//! their syscall paths. `libzkevm` exports `_start`, `read_input`, `write_output` and `abort`.
//! `sp1-zkvm`'s allocator manages a private region of its own, which `zkvm.ld` defines apart from
//! the guest's heap.
//!
//! That allocator is the SDK's `embedded-alloc`, a bump allocator, and `libzkevm` is built without
//! its default `exports` feature, which leaves its `zkvm_*` accelerators unexported: this crate
//! exports each one, calling `libzkevm`'s and then freeing everything it allocated.
//!
//! Some of `libzkevm`'s dependencies need `std`, which the stock toolchain builds from source
//! (`-Zbuild-std=std`) with upstream's `target_os = "zkvm"` port. `std` also supplies the panic
//! handler, which `vendor-archive.sh` keeps internal.

use zkevm::precompile::types::*;

/// Frees, when dropped, everything allocated from `sp1-zkvm`'s heap since it was created. Each
/// export below holds one for the length of its accelerator call: every pointer argument points to
/// caller memory, so nothing the accelerator allocated is used again.
struct HeapScope(usize);

impl HeapScope {
    fn new() -> Self {
        Self(sp1_zkvm::allocators::embedded::INNER_HEAP.mark())
    }
}

impl Drop for HeapScope {
    fn drop(&mut self) {
        unsafe { sp1_zkvm::allocators::embedded::INNER_HEAP.release(self.0) }
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_keccak256(
    data: *const u8,
    len: usize,
    output: *mut Keccak256Hash,
) -> i32 {
    let _heap = HeapScope::new();
    unsafe { zkevm::precompile::hash::zkvm_keccak256(data, len, output) }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_secp256k1_verify(
    msg: *const Secp256k1Hash,
    sig: *const Secp256k1Signature,
    pubkey: *const Secp256k1Pubkey,
    verified: *mut bool,
) -> i32 {
    let _heap = HeapScope::new();
    unsafe { zkevm::precompile::secp256k1::zkvm_secp256k1_verify(msg, sig, pubkey, verified) }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_secp256k1_ecrecover(
    msg: *const Secp256k1Hash,
    sig: *const Secp256k1Signature,
    recid: u8,
    output: *mut Secp256k1Pubkey,
) -> i32 {
    let _heap = HeapScope::new();
    unsafe { zkevm::precompile::secp256k1::zkvm_secp256k1_ecrecover(msg, sig, recid, output) }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_sha256(data: *const u8, len: usize, output: *mut Sha256Hash) -> i32 {
    let _heap = HeapScope::new();
    unsafe { zkevm::precompile::hash::zkvm_sha256(data, len, output) }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_ripemd160(
    data: *const u8,
    len: usize,
    output: *mut Ripemd160Hash,
) -> i32 {
    let _heap = HeapScope::new();
    unsafe { zkevm::precompile::hash::zkvm_ripemd160(data, len, output) }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_modexp(
    base: *const u8,
    base_len: usize,
    exp: *const u8,
    exp_len: usize,
    modulus: *const u8,
    mod_len: usize,
    output: *mut u8,
) -> i32 {
    let _heap = HeapScope::new();
    unsafe {
        zkevm::precompile::modexp::zkvm_modexp(
            base, base_len, exp, exp_len, modulus, mod_len, output,
        )
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_bn254_g1_add(
    p1: *const Bn254G1Point,
    p2: *const Bn254G1Point,
    result: *mut Bn254G1Point,
) -> i32 {
    let _heap = HeapScope::new();
    unsafe { zkevm::precompile::bn254::zkvm_bn254_g1_add(p1, p2, result) }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_bn254_g1_mul(
    point: *const Bn254G1Point,
    scalar: *const Bn254Scalar,
    result: *mut Bn254G1Point,
) -> i32 {
    let _heap = HeapScope::new();
    unsafe { zkevm::precompile::bn254::zkvm_bn254_g1_mul(point, scalar, result) }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_bn254_pairing(
    pairs: *const Bn254PairingPair,
    num_pairs: usize,
    verified: *mut bool,
) -> i32 {
    let _heap = HeapScope::new();
    unsafe { zkevm::precompile::bn254::zkvm_bn254_pairing(pairs, num_pairs, verified) }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_blake2f(
    rounds: u32,
    h: *mut Blake2fState,
    m: *const Blake2fMessage,
    t: *const Blake2fOffset,
    f: u8,
) -> i32 {
    let _heap = HeapScope::new();
    unsafe { zkevm::precompile::blake2f::zkvm_blake2f(rounds, h, m, t, f) }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_kzg_point_eval(
    commitment: *const KzgCommitment,
    z: *const KzgFieldElement,
    y: *const KzgFieldElement,
    proof: *const KzgProof,
    verified: *mut bool,
) -> i32 {
    let _heap = HeapScope::new();
    unsafe { zkevm::precompile::kzg::zkvm_kzg_point_eval(commitment, z, y, proof, verified) }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_bls12_g1_add(
    p1: *const Bls12381G1Point,
    p2: *const Bls12381G1Point,
    result: *mut Bls12381G1Point,
) -> i32 {
    let _heap = HeapScope::new();
    unsafe { zkevm::precompile::bls12_381::zkvm_bls12_g1_add(p1, p2, result) }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_bls12_g1_msm(
    pairs: *const Bls12381G1MsmPair,
    num_pairs: usize,
    result: *mut Bls12381G1Point,
) -> i32 {
    let _heap = HeapScope::new();
    unsafe { zkevm::precompile::bls12_381::zkvm_bls12_g1_msm(pairs, num_pairs, result) }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_bls12_g2_add(
    p1: *const Bls12381G2Point,
    p2: *const Bls12381G2Point,
    result: *mut Bls12381G2Point,
) -> i32 {
    let _heap = HeapScope::new();
    unsafe { zkevm::precompile::bls12_381::zkvm_bls12_g2_add(p1, p2, result) }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_bls12_g2_msm(
    pairs: *const Bls12381G2MsmPair,
    num_pairs: usize,
    result: *mut Bls12381G2Point,
) -> i32 {
    let _heap = HeapScope::new();
    unsafe { zkevm::precompile::bls12_381::zkvm_bls12_g2_msm(pairs, num_pairs, result) }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_bls12_pairing(
    pairs: *const Bls12381PairingPair,
    num_pairs: usize,
    verified: *mut bool,
) -> i32 {
    let _heap = HeapScope::new();
    unsafe { zkevm::precompile::bls12_381::zkvm_bls12_pairing(pairs, num_pairs, verified) }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_bls12_map_fp_to_g1(
    field_element: *const Bls12381Fp,
    result: *mut Bls12381G1Point,
) -> i32 {
    let _heap = HeapScope::new();
    unsafe { zkevm::precompile::bls12_381::zkvm_bls12_map_fp_to_g1(field_element, result) }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_bls12_map_fp2_to_g2(
    field_element: *const Bls12381Fp2,
    result: *mut Bls12381G2Point,
) -> i32 {
    let _heap = HeapScope::new();
    unsafe { zkevm::precompile::bls12_381::zkvm_bls12_map_fp2_to_g2(field_element, result) }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_secp256r1_verify(
    msg: *const Secp256r1Hash,
    sig: *const Secp256r1Signature,
    pubkey: *const Secp256r1Pubkey,
    verified: *mut bool,
) -> i32 {
    let _heap = HeapScope::new();
    unsafe { zkevm::precompile::secp256r1::zkvm_secp256r1_verify(msg, sig, pubkey, verified) }
}
