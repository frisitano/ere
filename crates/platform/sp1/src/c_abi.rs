//! The zkvm-standards C ABI, built on SP1's `libzkevm` without its `exports` feature, for a
//! staticlib (`staticlib/build.sh`): `libzkevm` then leaves its `zkvm_*` accelerators unexported,
//! and this module exports each one, calling `libzkevm`'s and then freeing everything it
//! allocated. `_start`, `read_input`, `write_output` and `abort` come from `libzkevm` itself.
//!
//! `sp1-zkvm`'s allocator manages a private region, which `staticlib/zkvm.ld` defines apart from
//! the guest's heap: the staticlib build patches in the bump allocator of
//! `staticlib/embedded-alloc` for its `embedded-alloc` heap, so it can be rewound.
//!
//! Some of `libzkevm`'s dependencies need `std`, which the stock toolchain builds from source
//! (`-Zbuild-std=std`) with upstream's `target_os = "zkvm"` port. `std` also supplies the panic
//! handler, which `vendor-archive.sh` keeps internal.

use ere_platform_core::abi::{
    Bls12G1MsmPair, Bls12G2MsmPair, Bls12PairingPair, Bn254PairingPair, Bytes, Status,
};
use sp1_libzkevm::precompile;
use sp1_zkvm::allocators::embedded::INNER_HEAP;

/// Frees, when dropped, everything allocated from `sp1-zkvm`'s heap since it was created. Each
/// export below holds one for the length of its accelerator call: every pointer argument points to
/// caller memory, so nothing the accelerator allocated is used again.
struct HeapScope(usize);

impl HeapScope {
    fn new() -> Self {
        Self(INNER_HEAP.mark())
    }
}

impl Drop for HeapScope {
    fn drop(&mut self) {
        unsafe { INNER_HEAP.release(self.0) }
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_keccak256(data: *const u8, len: usize, output: *mut Bytes<32>) -> Status {
    let _heap = HeapScope::new();
    Status(unsafe { precompile::hash::zkvm_keccak256(data, len, output.cast()) })
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_secp256k1_verify(
    msg: *const Bytes<32>,
    sig: *const Bytes<64>,
    pubkey: *const Bytes<64>,
    verified: *mut bool,
) -> Status {
    let _heap = HeapScope::new();
    Status(unsafe {
        precompile::secp256k1::zkvm_secp256k1_verify(
            msg.cast(),
            sig.cast(),
            pubkey.cast(),
            verified,
        )
    })
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_secp256k1_ecrecover(
    msg: *const Bytes<32>,
    sig: *const Bytes<64>,
    recid: u8,
    output: *mut Bytes<64>,
) -> Status {
    let _heap = HeapScope::new();
    Status(unsafe {
        precompile::secp256k1::zkvm_secp256k1_ecrecover(
            msg.cast(),
            sig.cast(),
            recid,
            output.cast(),
        )
    })
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_sha256(data: *const u8, len: usize, output: *mut Bytes<32>) -> Status {
    let _heap = HeapScope::new();
    Status(unsafe { precompile::hash::zkvm_sha256(data, len, output.cast()) })
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_ripemd160(data: *const u8, len: usize, output: *mut Bytes<32>) -> Status {
    let _heap = HeapScope::new();
    Status(unsafe { precompile::hash::zkvm_ripemd160(data, len, output.cast()) })
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
) -> Status {
    let _heap = HeapScope::new();
    Status(unsafe {
        precompile::modexp::zkvm_modexp(base, base_len, exp, exp_len, modulus, mod_len, output)
    })
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_bn254_g1_add(
    p1: *const Bytes<64>,
    p2: *const Bytes<64>,
    result: *mut Bytes<64>,
) -> Status {
    let _heap = HeapScope::new();
    Status(unsafe { precompile::bn254::zkvm_bn254_g1_add(p1.cast(), p2.cast(), result.cast()) })
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_bn254_g1_mul(
    point: *const Bytes<64>,
    scalar: *const Bytes<32>,
    result: *mut Bytes<64>,
) -> Status {
    let _heap = HeapScope::new();
    Status(unsafe {
        precompile::bn254::zkvm_bn254_g1_mul(point.cast(), scalar.cast(), result.cast())
    })
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_bn254_pairing(
    pairs: *const Bn254PairingPair,
    num_pairs: usize,
    verified: *mut bool,
) -> Status {
    let _heap = HeapScope::new();
    Status(unsafe { precompile::bn254::zkvm_bn254_pairing(pairs.cast(), num_pairs, verified) })
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_blake2f(
    rounds: u32,
    h: *mut Bytes<64>,
    m: *const Bytes<128>,
    t: *const Bytes<16>,
    f: u8,
) -> Status {
    let _heap = HeapScope::new();
    Status(unsafe { precompile::blake2f::zkvm_blake2f(rounds, h.cast(), m.cast(), t.cast(), f) })
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_kzg_point_eval(
    commitment: *const Bytes<48>,
    z: *const Bytes<32>,
    y: *const Bytes<32>,
    proof: *const Bytes<48>,
    verified: *mut bool,
) -> Status {
    let _heap = HeapScope::new();
    Status(unsafe {
        precompile::kzg::zkvm_kzg_point_eval(
            commitment.cast(),
            z.cast(),
            y.cast(),
            proof.cast(),
            verified,
        )
    })
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_bls12_g1_add(
    p1: *const Bytes<96>,
    p2: *const Bytes<96>,
    result: *mut Bytes<96>,
) -> Status {
    let _heap = HeapScope::new();
    Status(unsafe { precompile::bls12_381::zkvm_bls12_g1_add(p1.cast(), p2.cast(), result.cast()) })
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_bls12_g1_msm(
    pairs: *const Bls12G1MsmPair,
    num_pairs: usize,
    result: *mut Bytes<96>,
) -> Status {
    let _heap = HeapScope::new();
    Status(unsafe {
        precompile::bls12_381::zkvm_bls12_g1_msm(pairs.cast(), num_pairs, result.cast())
    })
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_bls12_g2_add(
    p1: *const Bytes<192>,
    p2: *const Bytes<192>,
    result: *mut Bytes<192>,
) -> Status {
    let _heap = HeapScope::new();
    Status(unsafe { precompile::bls12_381::zkvm_bls12_g2_add(p1.cast(), p2.cast(), result.cast()) })
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_bls12_g2_msm(
    pairs: *const Bls12G2MsmPair,
    num_pairs: usize,
    result: *mut Bytes<192>,
) -> Status {
    let _heap = HeapScope::new();
    Status(unsafe {
        precompile::bls12_381::zkvm_bls12_g2_msm(pairs.cast(), num_pairs, result.cast())
    })
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_bls12_pairing(
    pairs: *const Bls12PairingPair,
    num_pairs: usize,
    verified: *mut bool,
) -> Status {
    let _heap = HeapScope::new();
    Status(unsafe { precompile::bls12_381::zkvm_bls12_pairing(pairs.cast(), num_pairs, verified) })
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_bls12_map_fp_to_g1(
    field_element: *const Bytes<48>,
    result: *mut Bytes<96>,
) -> Status {
    let _heap = HeapScope::new();
    Status(unsafe {
        precompile::bls12_381::zkvm_bls12_map_fp_to_g1(field_element.cast(), result.cast())
    })
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_bls12_map_fp2_to_g2(
    field_element: *const Bytes<96>,
    result: *mut Bytes<192>,
) -> Status {
    let _heap = HeapScope::new();
    Status(unsafe {
        precompile::bls12_381::zkvm_bls12_map_fp2_to_g2(field_element.cast(), result.cast())
    })
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_secp256r1_verify(
    msg: *const Bytes<32>,
    sig: *const Bytes<64>,
    pubkey: *const Bytes<64>,
    verified: *mut bool,
) -> Status {
    let _heap = HeapScope::new();
    Status(unsafe {
        precompile::secp256r1::zkvm_secp256r1_verify(
            msg.cast(),
            sig.cast(),
            pubkey.cast(),
            verified,
        )
    })
}
