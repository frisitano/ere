//! Software control SDK library: every `zkvm_*` accelerator symbol, in plain RISC-V.
//!
//! `build.sh <zkvm> <out> software` links it in place of the vendor's accelerators and keeps the
//! vendor's runtime. The acceleration check compares a guest's cost against this control.
//!
//! Each symbol calls `revm-precompile`'s default `Crypto` with only its pure-Rust backends, the
//! same boundary ere's `zkvm_interface` test vectors were recorded at. `zkvm_secp256k1_*` use
//! `k256` directly, because the C interface returns a public key where `Crypto` returns an
//! address. Built for the generic target, it uses `std` through the same `sys_*` functions a
//! guest's `std` does, so its heap is the vendor's.

use core::slice;

use k256::ecdsa::{RecoveryId, Signature, VerifyingKey, signature::hazmat::PrehashVerifier};
use revm_precompile::{Crypto, DefaultCrypto};
use sha3::{Digest, Keccak256};
use zkvm_interface::*;

use ere_platform_zkvm as _;

const OK: zkvm_status = zkvm_status_ZKVM_EOK;
const FAIL: zkvm_status = zkvm_status_ZKVM_EFAIL;

/// Maps a result to a status, writing the value on success.
fn put<T>(result: Result<T, impl Sized>, out: &mut T) -> zkvm_status {
    match result {
        Ok(value) => {
            *out = value;
            OK
        }
        Err(_) => FAIL,
    }
}

fn split<const N: usize, const H: usize>(bytes: &[u8; N]) -> ([u8; H], [u8; H]) {
    (
        bytes[..H].try_into().unwrap(),
        bytes[H..].try_into().unwrap(),
    )
}

fn g2(bytes: &[u8; 192]) -> ([u8; 48], [u8; 48], [u8; 48], [u8; 48]) {
    let part = |i: usize| bytes[i * 48..(i + 1) * 48].try_into().unwrap();
    (part(0), part(1), part(2), part(3))
}

unsafe fn bytes<'a>(ptr: *const u8, len: usize) -> &'a [u8] {
    if len == 0 {
        &[]
    } else {
        unsafe { slice::from_raw_parts(ptr, len) }
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_keccak256(
    data: *const u8,
    len: usize,
    output: *mut zkvm_keccak256_hash,
) -> zkvm_status {
    let data = unsafe { bytes(data, len) };
    unsafe { (*output).data = Keccak256::digest(data).into() };
    OK
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_sha256(
    data: *const u8,
    len: usize,
    output: *mut zkvm_sha256_hash,
) -> zkvm_status {
    let data = unsafe { bytes(data, len) };
    unsafe { (*output).data = DefaultCrypto.sha256(data) };
    OK
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_ripemd160(
    data: *const u8,
    len: usize,
    output: *mut zkvm_ripemd160_hash,
) -> zkvm_status {
    let data = unsafe { bytes(data, len) };
    unsafe { (*output).data = DefaultCrypto.ripemd160(data) };
    OK
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_secp256k1_verify(
    msg: *const zkvm_secp256k1_hash,
    sig: *const zkvm_secp256k1_signature,
    pubkey: *const zkvm_secp256k1_pubkey,
    verified: *mut bool,
) -> zkvm_status {
    let (msg, sig, pubkey) = unsafe { (&(*msg).data, &(*sig).data, &(*pubkey).data) };
    let mut sec1 = [4u8; 65];
    sec1[1..].copy_from_slice(pubkey);
    let (Ok(key), Ok(sig)) = (
        VerifyingKey::from_sec1_bytes(&sec1),
        Signature::from_slice(sig),
    ) else {
        return FAIL;
    };
    unsafe { *verified = key.verify_prehash(msg, &sig).is_ok() };
    OK
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_secp256k1_ecrecover(
    msg: *const zkvm_secp256k1_hash,
    sig: *const zkvm_secp256k1_signature,
    recid: u8,
    output: *mut zkvm_secp256k1_pubkey,
) -> zkvm_status {
    let (msg, sig) = unsafe { (&(*msg).data, &(*sig).data) };
    let (Ok(mut sig), Some(mut recid)) = (Signature::from_slice(sig), RecoveryId::from_byte(recid))
    else {
        return FAIL;
    };
    // As `revm-precompile`'s `k256` backend: a high-s signature recovers with the flipped id.
    if let Some(normalized) = sig.normalize_s() {
        sig = normalized;
        recid = RecoveryId::from_byte(recid.to_byte() ^ 1).unwrap();
    }
    let Ok(key) = VerifyingKey::recover_from_prehash(msg, &sig, recid) else {
        return FAIL;
    };
    let point = key.to_encoded_point(false);
    unsafe { (*output).data.copy_from_slice(&point.as_bytes()[1..]) };
    OK
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
) -> zkvm_status {
    let (base, exp, modulus) = unsafe {
        (
            bytes(base, base_len),
            bytes(exp, exp_len),
            bytes(modulus, mod_len),
        )
    };
    let Ok(result) = DefaultCrypto.modexp(base, exp, modulus) else {
        return FAIL;
    };
    // `revm` returns the value left-padded to the modulus length.
    let output = unsafe { slice::from_raw_parts_mut(output, mod_len) };
    output.fill(0);
    let len = result.len().min(mod_len);
    output[mod_len - len..].copy_from_slice(&result[result.len() - len..]);
    OK
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_bn254_g1_add(
    p1: *const zkvm_bn254_g1_point,
    p2: *const zkvm_bn254_g1_point,
    result: *mut zkvm_bn254_g1_point,
) -> zkvm_status {
    let (p1, p2) = unsafe { (&(*p1).data, &(*p2).data) };
    unsafe { put(DefaultCrypto.bn254_g1_add(p1, p2), &mut (*result).data) }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_bn254_g1_mul(
    point: *const zkvm_bn254_g1_point,
    scalar: *const zkvm_bn254_scalar,
    result: *mut zkvm_bn254_g1_point,
) -> zkvm_status {
    let (point, scalar) = unsafe { (&(*point).data, &(*scalar).data) };
    unsafe { put(DefaultCrypto.bn254_g1_mul(point, scalar), &mut (*result).data) }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_bn254_pairing(
    pairs: *const zkvm_bn254_pairing_pair,
    num_pairs: usize,
    verified: *mut bool,
) -> zkvm_status {
    let pairs = unsafe { slice::from_raw_parts(pairs, num_pairs) };
    let pairs: Vec<(&[u8], &[u8])> = pairs
        .iter()
        .map(|pair| (&pair.g1.data[..], &pair.g2.data[..]))
        .collect();
    unsafe { put(DefaultCrypto.bn254_pairing_check(&pairs), &mut *verified) }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_blake2f(
    rounds: u32,
    h: *mut zkvm_blake2f_state,
    m: *const zkvm_blake2f_message,
    t: *const zkvm_blake2f_offset,
    f: u8,
) -> zkvm_status {
    let words = |bytes: &[u8]| -> Vec<u64> {
        bytes
            .chunks_exact(8)
            .map(|chunk| u64::from_le_bytes(chunk.try_into().unwrap()))
            .collect()
    };
    let (state, m, t) = unsafe { (&mut (*h).data, &(*m).data, &(*t).data) };
    let mut hw: [u64; 8] = words(state).try_into().unwrap();
    let mw: [u64; 16] = words(m).try_into().unwrap();
    let tw: [u64; 2] = words(t).try_into().unwrap();
    DefaultCrypto.blake2_compress(rounds, &mut hw, &mw, &tw, f != 0);
    for (chunk, word) in state.chunks_exact_mut(8).zip(hw) {
        chunk.copy_from_slice(&word.to_le_bytes());
    }
    OK
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_kzg_point_eval(
    commitment: *const zkvm_kzg_commitment,
    z: *const zkvm_kzg_field_element,
    y: *const zkvm_kzg_field_element,
    proof: *const zkvm_kzg_proof,
    verified: *mut bool,
) -> zkvm_status {
    let (commitment, z, y, proof) =
        unsafe { (&(*commitment).data, &(*z).data, &(*y).data, &(*proof).data) };
    unsafe { *verified = DefaultCrypto.verify_kzg_proof(z, y, commitment, proof).is_ok() };
    OK
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_bls12_g1_add(
    p1: *const zkvm_bls12_381_g1_point,
    p2: *const zkvm_bls12_381_g1_point,
    result: *mut zkvm_bls12_381_g1_point,
) -> zkvm_status {
    let (p1, p2) = unsafe { (split(&(*p1).data), split(&(*p2).data)) };
    unsafe { put(DefaultCrypto.bls12_381_g1_add(p1, p2), &mut (*result).data) }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_bls12_g1_msm(
    pairs: *const zkvm_bls12_381_g1_msm_pair,
    num_pairs: usize,
    result: *mut zkvm_bls12_381_g1_point,
) -> zkvm_status {
    let pairs = unsafe { slice::from_raw_parts(pairs, num_pairs) };
    let mut pairs = pairs
        .iter()
        .map(|pair| Ok((split(&pair.point.data), pair.scalar.data)));
    unsafe { put(DefaultCrypto.bls12_381_g1_msm(&mut pairs), &mut (*result).data) }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_bls12_g2_add(
    p1: *const zkvm_bls12_381_g2_point,
    p2: *const zkvm_bls12_381_g2_point,
    result: *mut zkvm_bls12_381_g2_point,
) -> zkvm_status {
    let (p1, p2) = unsafe { (g2(&(*p1).data), g2(&(*p2).data)) };
    unsafe { put(DefaultCrypto.bls12_381_g2_add(p1, p2), &mut (*result).data) }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_bls12_g2_msm(
    pairs: *const zkvm_bls12_381_g2_msm_pair,
    num_pairs: usize,
    result: *mut zkvm_bls12_381_g2_point,
) -> zkvm_status {
    let pairs = unsafe { slice::from_raw_parts(pairs, num_pairs) };
    let mut pairs = pairs
        .iter()
        .map(|pair| Ok((g2(&pair.point.data), pair.scalar.data)));
    unsafe { put(DefaultCrypto.bls12_381_g2_msm(&mut pairs), &mut (*result).data) }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_bls12_pairing(
    pairs: *const zkvm_bls12_381_pairing_pair,
    num_pairs: usize,
    verified: *mut bool,
) -> zkvm_status {
    let pairs = unsafe { slice::from_raw_parts(pairs, num_pairs) };
    let pairs: Vec<_> = pairs
        .iter()
        .map(|pair| (split(&pair.g1.data), g2(&pair.g2.data)))
        .collect();
    unsafe { put(DefaultCrypto.bls12_381_pairing_check(&pairs), &mut *verified) }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_bls12_map_fp_to_g1(
    field_element: *const zkvm_bls12_381_fp,
    result: *mut zkvm_bls12_381_g1_point,
) -> zkvm_status {
    let fp = unsafe { &(*field_element).data };
    unsafe { put(DefaultCrypto.bls12_381_fp_to_g1(fp), &mut (*result).data) }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_bls12_map_fp2_to_g2(
    field_element: *const zkvm_bls12_381_fp2,
    result: *mut zkvm_bls12_381_g2_point,
) -> zkvm_status {
    let fp2 = unsafe { split(&(*field_element).data) };
    unsafe { put(DefaultCrypto.bls12_381_fp2_to_g2(fp2), &mut (*result).data) }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_secp256r1_verify(
    msg: *const zkvm_secp256r1_hash,
    sig: *const zkvm_secp256r1_signature,
    pubkey: *const zkvm_secp256r1_pubkey,
    verified: *mut bool,
) -> zkvm_status {
    let (msg, sig, pubkey) = unsafe { (&(*msg).data, &(*sig).data, &(*pubkey).data) };
    unsafe { *verified = DefaultCrypto.secp256r1_verify_signature(msg, sig, pubkey) };
    OK
}

#[path = "../../shims/u256.rs"]
mod u256;

/// Every `zkvm_u256_*` in software.
mod u256_ops {
    pub use super::u256::sw::*;
}
