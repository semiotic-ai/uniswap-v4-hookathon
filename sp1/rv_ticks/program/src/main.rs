//! A simple program to be proven inside the zkVM.

#![no_main]
sp1_zkvm::entrypoint!(main);
use alloy_sol_types::{sol, SolType};
use fixed::types::I15F17 as Fixed;

/// The public values encoded as a tuple that can be easily deserialized inside Solidity.
type PublicValuesTuple = sol! {
    tuple(bytes[], bytes, bytes, bytes, bytes)
};

pub fn main() {
    // NOTE: values of n larger than 186 will overflow the u128 type,
    // resulting in output that doesn't match fibonacci sequence.
    // However, the resulting proof will still be valid!
    let values = sp1_zkvm::io::read::<Vec<[u8; 4]>>();
    let n_inv_sqrt = sp1_zkvm::io::read::<[u8; 4]>();
    let n1_inv = sp1_zkvm::io::read::<[u8; 4]>();
    let (s2_bytes, n_bytes) = tick_volatility2(values.clone(), n_inv_sqrt, n1_inv);
    println!("s2 bytes {:?}", s2_bytes);

    // Encocde the public values of the program.
    let bytes =
        PublicValuesTuple::abi_encode(&(values.clone(), n_inv_sqrt, n1_inv, s2_bytes, n_bytes));

    // Commit to the public values of the program.
    sp1_zkvm::io::commit_slice(&bytes);
}

pub fn tick_volatility2(
    values: Vec<[u8; 4]>,
    n_inv_sqrt: [u8; 4],
    n1_inv: [u8; 4],
) -> ([u8; 4], [u8; 4]) {
    let n = Fixed::from_num(values.len());
    let n_inv_sqrt = Fixed::from_be_bytes(n_inv_sqrt);
    let n1_inv = Fixed::from_be_bytes(n1_inv);

    let mut ticks_prev = Fixed::from_be_bytes(values[0]);
    let (sum_u, sum_u2) =
        values
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

    (s2_bytes, n_bytes)
}
