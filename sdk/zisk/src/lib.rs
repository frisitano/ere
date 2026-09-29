//! ZisK SDK library, from `ziskos` built for a target with `os = "zkvm"` and `vendor = "zisk"`, so
//! it selects its guest paths (`cfg(zisk_guest)`), and with its `staticlib` feature:
//!
//! - `_start` calls `int main(void)` and passes its return value to the ZisK exit syscall.
//! - `ziskos` allocates from a private buffer, never from the guest's heap (`_heap_start` to
//!   `_heap_end`). Each exported function below rewinds that buffer first: calls do not reenter,
//!   so nothing `ziskos` allocates outlives the call. These are the wrappers of ZisK's own
//!   `ziskos-staticlib` crate (`wrap_export!`, tag `v1.3.0-alpha`), which does not build as a
//!   Rust library.
//!
//! `abort`, which `ziskos` does not export, is defined here.

#![no_std]

use zisk_zkvm_interface::{
    zkvm_blake2f_message, zkvm_blake2f_offset, zkvm_blake2f_state, zkvm_bls12_381_fp,
    zkvm_bls12_381_fp2, zkvm_bls12_381_g1_msm_pair, zkvm_bls12_381_g1_point,
    zkvm_bls12_381_g2_msm_pair, zkvm_bls12_381_g2_point, zkvm_bls12_381_pairing_pair,
    zkvm_bn254_g1_point, zkvm_bn254_pairing_pair, zkvm_bn254_scalar, zkvm_keccak256_hash,
    zkvm_kzg_commitment, zkvm_kzg_field_element, zkvm_kzg_proof, zkvm_ripemd160_hash,
    zkvm_secp256k1_hash, zkvm_secp256k1_pubkey, zkvm_secp256k1_signature, zkvm_secp256r1_hash,
    zkvm_secp256r1_pubkey, zkvm_secp256r1_signature, zkvm_sha256_hash, zkvm_status,
};

unsafe extern "C" {
    /// Rewinds `ziskos`'s private heap (`ziskos` exports it only with the `staticlib` feature).
    fn reset_sys_alloc();
}

/// Exports `$name`, which rewinds `ziskos`'s private heap and calls `ziskos`'s `$name` at `$path`.
macro_rules! export {
    ($( fn $name:ident($($arg:ident: $ty:ty),* $(,)?) $(-> $ret:ty)? => $($path:ident)::+; )*) => {$(
        #[unsafe(no_mangle)]
        unsafe extern "C" fn $name($($arg: $ty),*) $(-> $ret)? {
            use $($path)::+::$name as inner;
            unsafe {
                reset_sys_alloc();
                inner($($arg),*)
            }
        }
    )*};
}

export! {
    fn zkvm_keccak256(data: *const u8, len: usize, output: *mut zkvm_keccak256_hash) -> zkvm_status
        => ziskos::zisklib::zkvm_accelerators;
    fn zkvm_secp256k1_verify(msg: *const zkvm_secp256k1_hash, sig: *const zkvm_secp256k1_signature, pubkey: *const zkvm_secp256k1_pubkey, verified: *mut bool) -> zkvm_status
        => ziskos::zisklib::zkvm_accelerators;
    fn zkvm_secp256k1_ecrecover(msg: *const zkvm_secp256k1_hash, sig: *const zkvm_secp256k1_signature, recid: u8, output: *mut zkvm_secp256k1_pubkey) -> zkvm_status
        => ziskos::zisklib::zkvm_accelerators;
    fn zkvm_sha256(data: *const u8, len: usize, output: *mut zkvm_sha256_hash) -> zkvm_status
        => ziskos::zisklib::zkvm_accelerators;
    fn zkvm_ripemd160(data: *const u8, len: usize, output: *mut zkvm_ripemd160_hash) -> zkvm_status
        => ziskos::zisklib::zkvm_accelerators;
    fn zkvm_modexp(base: *const u8, base_len: usize, exp: *const u8, exp_len: usize, modulus: *const u8, mod_len: usize, output: *mut u8) -> zkvm_status
        => ziskos::zisklib::zkvm_accelerators;
    fn zkvm_bn254_g1_add(p1: *const zkvm_bn254_g1_point, p2: *const zkvm_bn254_g1_point, result: *mut zkvm_bn254_g1_point) -> zkvm_status
        => ziskos::zisklib::zkvm_accelerators;
    fn zkvm_bn254_g1_mul(point: *const zkvm_bn254_g1_point, scalar: *const zkvm_bn254_scalar, result: *mut zkvm_bn254_g1_point) -> zkvm_status
        => ziskos::zisklib::zkvm_accelerators;
    fn zkvm_bn254_pairing(pairs: *const zkvm_bn254_pairing_pair, num_pairs: usize, verified: *mut bool) -> zkvm_status
        => ziskos::zisklib::zkvm_accelerators;
    fn zkvm_blake2f(rounds: u32, h: *mut zkvm_blake2f_state, m: *const zkvm_blake2f_message, t: *const zkvm_blake2f_offset, f: u8) -> zkvm_status
        => ziskos::zisklib::zkvm_accelerators;
    fn zkvm_kzg_point_eval(commitment: *const zkvm_kzg_commitment, z: *const zkvm_kzg_field_element, y: *const zkvm_kzg_field_element, proof: *const zkvm_kzg_proof, verified: *mut bool) -> zkvm_status
        => ziskos::zisklib::zkvm_accelerators;
    fn zkvm_bls12_g1_add(p1: *const zkvm_bls12_381_g1_point, p2: *const zkvm_bls12_381_g1_point, result: *mut zkvm_bls12_381_g1_point) -> zkvm_status
        => ziskos::zisklib::zkvm_accelerators;
    fn zkvm_bls12_g1_msm(pairs: *const zkvm_bls12_381_g1_msm_pair, num_pairs: usize, result: *mut zkvm_bls12_381_g1_point) -> zkvm_status
        => ziskos::zisklib::zkvm_accelerators;
    fn zkvm_bls12_g2_add(p1: *const zkvm_bls12_381_g2_point, p2: *const zkvm_bls12_381_g2_point, result: *mut zkvm_bls12_381_g2_point) -> zkvm_status
        => ziskos::zisklib::zkvm_accelerators;
    fn zkvm_bls12_g2_msm(pairs: *const zkvm_bls12_381_g2_msm_pair, num_pairs: usize, result: *mut zkvm_bls12_381_g2_point) -> zkvm_status
        => ziskos::zisklib::zkvm_accelerators;
    fn zkvm_bls12_pairing(pairs: *const zkvm_bls12_381_pairing_pair, num_pairs: usize, verified: *mut bool) -> zkvm_status
        => ziskos::zisklib::zkvm_accelerators;
    fn zkvm_bls12_map_fp_to_g1(field_element: *const zkvm_bls12_381_fp, result: *mut zkvm_bls12_381_g1_point) -> zkvm_status
        => ziskos::zisklib::zkvm_accelerators;
    fn zkvm_bls12_map_fp2_to_g2(field_element: *const zkvm_bls12_381_fp2, result: *mut zkvm_bls12_381_g2_point) -> zkvm_status
        => ziskos::zisklib::zkvm_accelerators;
    fn zkvm_secp256r1_verify(msg: *const zkvm_secp256r1_hash, sig: *const zkvm_secp256r1_signature, pubkey: *const zkvm_secp256r1_pubkey, verified: *mut bool) -> zkvm_status
        => ziskos::zisklib::zkvm_accelerators;
    // `read_input` returns ZisK's input region, not heap memory, so the rewind cannot reclaim it.
    fn read_input(buf_ptr: *mut *const u8, buf_size: *mut usize) => ziskos::zisklib::zkvm_io;
    fn write_output(output: *const u8, size: usize) => ziskos::zisklib::zkvm_io;
}

/// Failed termination: ZisK's exit syscall, as `_start` uses it for `main`'s return value, with
/// exit code 1.
#[unsafe(no_mangle)]
extern "C" fn abort() -> ! {
    unsafe { core::arch::asm!("ecall", in("a7") 93, in("a0") 1, options(noreturn)) }
}

// Panics raised inside `ziskos` itself. `vendor-archive.sh` keeps this handler internal.
#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    abort()
}
