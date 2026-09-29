//! The types of the [zkvm-standards] C ABI (`zkvm_accelerators.h`), shared by the guest's side,
//! which calls the `zkvm_*` accelerators, and each zkVM's side, which exports them.
//!
//! [zkvm-standards]: https://github.com/eth-act/zkvm-standards

use core::ffi::c_int;

/// `zkvm_status`: what an accelerator returns.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Status(pub c_int);

impl Status {
    /// `ZKVM_EOK`: success.
    pub const OK: Self = Self(0);
    /// `ZKVM_EFAIL`: failure.
    pub const FAIL: Self = Self(-1);
}

/// `N` bytes aligned to 8: the standard's `zkvm_bytes_N`, which every hash, point, scalar and field
/// element is.
#[repr(C, align(8))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Bytes<const N: usize>(pub [u8; N]);

impl<const N: usize> From<[u8; N]> for Bytes<N> {
    fn from(bytes: [u8; N]) -> Self {
        Self(bytes)
    }
}

/// `zkvm_bn254_pairing_pair`: a BN254 G1 point and G2 point.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bn254PairingPair {
    pub g1: Bytes<64>,
    pub g2: Bytes<128>,
}

/// `zkvm_bls12_381_g1_msm_pair`: a BLS12-381 G1 point and scalar.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bls12G1MsmPair {
    pub point: Bytes<96>,
    pub scalar: Bytes<32>,
}

/// `zkvm_bls12_381_g2_msm_pair`: a BLS12-381 G2 point and scalar.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bls12G2MsmPair {
    pub point: Bytes<192>,
    pub scalar: Bytes<32>,
}

/// `zkvm_bls12_381_pairing_pair`: a BLS12-381 G1 point and G2 point.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bls12PairingPair {
    pub g1: Bytes<96>,
    pub g2: Bytes<192>,
}

// The layouts of the standard's structs.
const _: () = {
    assert!(align_of::<Bytes<16>>() == 8);
    assert!(size_of::<Bn254PairingPair>() == 192);
    assert!(size_of::<Bls12G1MsmPair>() == 128);
    assert!(size_of::<Bls12G2MsmPair>() == 224);
    assert!(size_of::<Bls12PairingPair>() == 288);
};
