//! A simple program to be proven inside the zkVM.

#![no_main]
sp1_zkvm::entrypoint!(main);
use fixed::types::I15F17 as Fixed;
use alloy_sol_types::{sol, SolType};
use tiny_keccak::{Hasher, Sha3};

include!("data.rs");

/// The public values encoded as a tuple that can be easily deserialized inside Solidity.
type PublicValuesTuple = sol! {
    tuple(bytes[], bytes, bytes, bytes, bytes)
};


pub fn main() {
    // NOTE: values of n larger than 186 will overflow the u128 type,
    // resulting in output that doesn't match fibonacci sequence.
    // However, the resulting proof will still be valid!
    let n_inv_sqrt = sp1_zkvm::io::read::<[u8; 4]>();
    let n1_inv = sp1_zkvm::io::read::<[u8; 4]>();
    let (s2_bytes, n_bytes, digest) = tick_volatility2( n_inv_sqrt, n1_inv);


    // Encocde the public values of the program.
//    let bytes = PublicValuesTuple::abi_encode(&(values.clone(), n_inv_sqrt, n1_inv, s2_bytes, n_bytes));

    // Commit to the public values of the program.
    sp1_zkvm::io::commit_slice(&s2_bytes);
    sp1_zkvm::io::commit_slice(&n_bytes);
    sp1_zkvm::io::commit_slice(&digest);
}

pub fn tick_volatility2(
    n_inv_sqrt: [u8; 4],
    n1_inv: [u8; 4],
) -> ([u8; 4], [u8; 4], [u8; 32]) {
    let n = Fixed::from_num(DATA.len());
    let n_inv_sqrt = Fixed::from_be_bytes(n_inv_sqrt);
    let n1_inv = Fixed::from_be_bytes(n1_inv);

    let mut ticks_prev = Fixed::from_be_bytes(DATA[0]);
    let (sum_u, sum_u2) =
        DATA
            .iter()
            .skip(1)
            .fold((Fixed::ZERO, Fixed::ZERO), |(sum_u, sum_u2), val| {
                let ticks_curr = Fixed::from_be_bytes(*val);
                let delta = ticks_curr - ticks_prev;
                ticks_prev = ticks_curr;
                (sum_u + delta * n_inv_sqrt, sum_u2 + delta * delta * n1_inv)
            });

    let s2_bytes = Fixed::to_be_bytes(sum_u2 - (sum_u * sum_u) * n1_inv);
    let n_bytes = Fixed::to_be_bytes(n);
    
    let mut sha3 = Sha3::v256();
    let mut output = [0u8; 32];
    DATA.iter().for_each(|x| sha3.update(x));
    sha3.finalize(&mut output);

    (s2_bytes, n_bytes, output)
}
