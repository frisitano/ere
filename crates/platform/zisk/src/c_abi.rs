//! The zkvm-standards C ABI, built on `ziskos` with its `staticlib` feature, for a staticlib
//! (`staticlib/build.sh`): `ziskos` then allocates from a private buffer, never from the guest's
//! heap (`_heap_start` to `_heap_end`), and leaves these functions to this module, each of which
//! rewinds that buffer first. Calls do not reenter, so nothing `ziskos` allocates outlives the
//! call. These are the wrappers of ZisK's own `ziskos-staticlib` crate (`wrap_export!`, tag
//! `v1.3.0-alpha`), which does not build as a Rust library.
//!
//! `_start`, from `ziskos`, calls `int main(void)` and passes its return value to the ZisK exit
//! syscall. `abort`, which `ziskos` does not export, and the panic handler are defined here.

use ere_platform_core::abi::{
    Bls12G1MsmPair, Bls12G2MsmPair, Bls12PairingPair, Bn254PairingPair, Bytes, Status,
};

unsafe extern "C" {
    /// Rewinds `ziskos`'s private heap (`ziskos` exports it only with the `staticlib` feature).
    fn reset_sys_alloc();
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_keccak256(data: *const u8, len: usize, output: *mut Bytes<32>) -> Status {
    unsafe {
        reset_sys_alloc();
        Status(ziskos::zisklib::zkvm_accelerators::zkvm_keccak256(
            data,
            len,
            output.cast(),
        ))
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_secp256k1_verify(
    msg: *const Bytes<32>,
    sig: *const Bytes<64>,
    pubkey: *const Bytes<64>,
    verified: *mut bool,
) -> Status {
    unsafe {
        reset_sys_alloc();
        Status(ziskos::zisklib::zkvm_accelerators::zkvm_secp256k1_verify(
            msg.cast(),
            sig.cast(),
            pubkey.cast(),
            verified,
        ))
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_secp256k1_ecrecover(
    msg: *const Bytes<32>,
    sig: *const Bytes<64>,
    recid: u8,
    output: *mut Bytes<64>,
) -> Status {
    unsafe {
        reset_sys_alloc();
        Status(
            ziskos::zisklib::zkvm_accelerators::zkvm_secp256k1_ecrecover(
                msg.cast(),
                sig.cast(),
                recid,
                output.cast(),
            ),
        )
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_sha256(data: *const u8, len: usize, output: *mut Bytes<32>) -> Status {
    unsafe {
        reset_sys_alloc();
        Status(ziskos::zisklib::zkvm_accelerators::zkvm_sha256(
            data,
            len,
            output.cast(),
        ))
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_ripemd160(data: *const u8, len: usize, output: *mut Bytes<32>) -> Status {
    unsafe {
        reset_sys_alloc();
        Status(ziskos::zisklib::zkvm_accelerators::zkvm_ripemd160(
            data,
            len,
            output.cast(),
        ))
    }
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
    unsafe {
        reset_sys_alloc();
        Status(ziskos::zisklib::zkvm_accelerators::zkvm_modexp(
            base, base_len, exp, exp_len, modulus, mod_len, output,
        ))
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_bn254_g1_add(
    p1: *const Bytes<64>,
    p2: *const Bytes<64>,
    result: *mut Bytes<64>,
) -> Status {
    unsafe {
        reset_sys_alloc();
        Status(ziskos::zisklib::zkvm_accelerators::zkvm_bn254_g1_add(
            p1.cast(),
            p2.cast(),
            result.cast(),
        ))
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_bn254_g1_mul(
    point: *const Bytes<64>,
    scalar: *const Bytes<32>,
    result: *mut Bytes<64>,
) -> Status {
    unsafe {
        reset_sys_alloc();
        Status(ziskos::zisklib::zkvm_accelerators::zkvm_bn254_g1_mul(
            point.cast(),
            scalar.cast(),
            result.cast(),
        ))
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_bn254_pairing(
    pairs: *const Bn254PairingPair,
    num_pairs: usize,
    verified: *mut bool,
) -> Status {
    unsafe {
        reset_sys_alloc();
        Status(ziskos::zisklib::zkvm_accelerators::zkvm_bn254_pairing(
            pairs.cast(),
            num_pairs,
            verified,
        ))
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_blake2f(
    rounds: u32,
    h: *mut Bytes<64>,
    m: *const Bytes<128>,
    t: *const Bytes<16>,
    f: u8,
) -> Status {
    unsafe {
        reset_sys_alloc();
        Status(ziskos::zisklib::zkvm_accelerators::zkvm_blake2f(
            rounds,
            h.cast(),
            m.cast(),
            t.cast(),
            f,
        ))
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_kzg_point_eval(
    commitment: *const Bytes<48>,
    z: *const Bytes<32>,
    y: *const Bytes<32>,
    proof: *const Bytes<48>,
    verified: *mut bool,
) -> Status {
    unsafe {
        reset_sys_alloc();
        Status(ziskos::zisklib::zkvm_accelerators::zkvm_kzg_point_eval(
            commitment.cast(),
            z.cast(),
            y.cast(),
            proof.cast(),
            verified,
        ))
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_bls12_g1_add(
    p1: *const Bytes<96>,
    p2: *const Bytes<96>,
    result: *mut Bytes<96>,
) -> Status {
    unsafe {
        reset_sys_alloc();
        Status(ziskos::zisklib::zkvm_accelerators::zkvm_bls12_g1_add(
            p1.cast(),
            p2.cast(),
            result.cast(),
        ))
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_bls12_g1_msm(
    pairs: *const Bls12G1MsmPair,
    num_pairs: usize,
    result: *mut Bytes<96>,
) -> Status {
    unsafe {
        reset_sys_alloc();
        Status(ziskos::zisklib::zkvm_accelerators::zkvm_bls12_g1_msm(
            pairs.cast(),
            num_pairs,
            result.cast(),
        ))
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_bls12_g2_add(
    p1: *const Bytes<192>,
    p2: *const Bytes<192>,
    result: *mut Bytes<192>,
) -> Status {
    unsafe {
        reset_sys_alloc();
        Status(ziskos::zisklib::zkvm_accelerators::zkvm_bls12_g2_add(
            p1.cast(),
            p2.cast(),
            result.cast(),
        ))
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_bls12_g2_msm(
    pairs: *const Bls12G2MsmPair,
    num_pairs: usize,
    result: *mut Bytes<192>,
) -> Status {
    unsafe {
        reset_sys_alloc();
        Status(ziskos::zisklib::zkvm_accelerators::zkvm_bls12_g2_msm(
            pairs.cast(),
            num_pairs,
            result.cast(),
        ))
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_bls12_pairing(
    pairs: *const Bls12PairingPair,
    num_pairs: usize,
    verified: *mut bool,
) -> Status {
    unsafe {
        reset_sys_alloc();
        Status(ziskos::zisklib::zkvm_accelerators::zkvm_bls12_pairing(
            pairs.cast(),
            num_pairs,
            verified,
        ))
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_bls12_map_fp_to_g1(
    field_element: *const Bytes<48>,
    result: *mut Bytes<96>,
) -> Status {
    unsafe {
        reset_sys_alloc();
        Status(ziskos::zisklib::zkvm_accelerators::zkvm_bls12_map_fp_to_g1(
            field_element.cast(),
            result.cast(),
        ))
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_bls12_map_fp2_to_g2(
    field_element: *const Bytes<96>,
    result: *mut Bytes<192>,
) -> Status {
    unsafe {
        reset_sys_alloc();
        Status(
            ziskos::zisklib::zkvm_accelerators::zkvm_bls12_map_fp2_to_g2(
                field_element.cast(),
                result.cast(),
            ),
        )
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn zkvm_secp256r1_verify(
    msg: *const Bytes<32>,
    sig: *const Bytes<64>,
    pubkey: *const Bytes<64>,
    verified: *mut bool,
) -> Status {
    unsafe {
        reset_sys_alloc();
        Status(ziskos::zisklib::zkvm_accelerators::zkvm_secp256r1_verify(
            msg.cast(),
            sig.cast(),
            pubkey.cast(),
            verified,
        ))
    }
}

// `read_input` returns ZisK's input region, not heap memory, so the rewind cannot reclaim it.
#[unsafe(no_mangle)]
unsafe extern "C" fn read_input(buf_ptr: *mut *const u8, buf_size: *mut usize) {
    unsafe {
        reset_sys_alloc();
        ziskos::zisklib::zkvm_io::read_input(buf_ptr, buf_size)
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn write_output(output: *const u8, size: usize) {
    unsafe {
        reset_sys_alloc();
        ziskos::zisklib::zkvm_io::write_output(output, size)
    }
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
