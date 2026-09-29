//! SP1 SDK library. Everything comes from SP1's `libzkevm`, built for a target with
//! `os = "zkvm"` and `vendor = "succinct"` so `sp1-zkvm` and SP1's patched crypto crates select
//! their syscall paths. `libzkevm` exports `_start`, `read_input`, `write_output`, every `zkvm_*`
//! symbol and `abort`. `sp1-zkvm`'s allocator manages its own region, from `_end` in `zkvm.ld`.
//!
//! That allocator is the SDK's `embedded-alloc`, a bump allocator, and each `zkvm_*` accelerator
//! (`wrapped.txt`) is wrapped to free, on return, everything it allocated: `vendor-archive.sh`
//! renames `libzkevm`'s `f` to `__zkvm_vendor_f` and this crate's `__zkvm_wrap_f` to `f`.
//!
//! Some of `libzkevm`'s dependencies need `std`, which the stock toolchain builds from source
//! (`-Zbuild-std=std`) with upstream's `target_os = "zkvm"` port. `std` also supplies the panic
//! handler, which `vendor-archive.sh` keeps internal.

use zkevm as _;

/// Defines `__zkvm_wrap_$name`, which calls `libzkevm`'s `$name` and then rewinds `sp1-zkvm`'s heap.
macro_rules! wrap {
    ($( fn $name:ident($($arg:ident: $ty:ty),* $(,)?); )*) => {$(
        const _: () = {
            unsafe extern "C" {
                #[link_name = concat!("__zkvm_vendor_", stringify!($name))]
                fn vendor($($arg: $ty),*) -> i32;
            }

            #[unsafe(export_name = concat!("__zkvm_wrap_", stringify!($name)))]
            unsafe extern "C" fn wrapper($($arg: $ty),*) -> i32 {
                let heap = &sp1_zkvm::allocators::embedded::INNER_HEAP;
                let mark = heap.mark();
                unsafe {
                    let status = vendor($($arg),*);
                    // The accelerator's outputs are in caller memory; nothing it allocated is used
                    // again.
                    heap.release(mark);
                    status
                }
            }
        };
    )*};
}

// The zkvm-standards accelerators; every pointer argument is a pointer to caller memory.
wrap! {
    fn zkvm_keccak256(data: *const u8, len: usize, output: *mut u8);
    fn zkvm_secp256k1_verify(msg: *const u8, sig: *const u8, pubkey: *const u8, verified: *mut bool);
    fn zkvm_secp256k1_ecrecover(msg: *const u8, sig: *const u8, recid: u8, output: *mut u8);
    fn zkvm_sha256(data: *const u8, len: usize, output: *mut u8);
    fn zkvm_ripemd160(data: *const u8, len: usize, output: *mut u8);
    fn zkvm_modexp(base: *const u8, base_len: usize, exp: *const u8, exp_len: usize, modulus: *const u8, mod_len: usize, output: *mut u8);
    fn zkvm_bn254_g1_add(p1: *const u8, p2: *const u8, result: *mut u8);
    fn zkvm_bn254_g1_mul(point: *const u8, scalar: *const u8, result: *mut u8);
    fn zkvm_bn254_pairing(pairs: *const u8, num_pairs: usize, verified: *mut bool);
    fn zkvm_blake2f(rounds: u32, h: *mut u8, m: *const u8, t: *const u8, f: u8);
    fn zkvm_kzg_point_eval(commitment: *const u8, z: *const u8, y: *const u8, proof: *const u8, verified: *mut bool);
    fn zkvm_bls12_g1_add(p1: *const u8, p2: *const u8, result: *mut u8);
    fn zkvm_bls12_g1_msm(pairs: *const u8, num_pairs: usize, result: *mut u8);
    fn zkvm_bls12_g2_add(p1: *const u8, p2: *const u8, result: *mut u8);
    fn zkvm_bls12_g2_msm(pairs: *const u8, num_pairs: usize, result: *mut u8);
    fn zkvm_bls12_pairing(pairs: *const u8, num_pairs: usize, verified: *mut bool);
    fn zkvm_bls12_map_fp_to_g1(field_element: *const u8, result: *mut u8);
    fn zkvm_bls12_map_fp2_to_g2(field_element: *const u8, result: *mut u8);
    fn zkvm_secp256r1_verify(msg: *const u8, sig: *const u8, pubkey: *const u8, verified: *mut bool);
}
