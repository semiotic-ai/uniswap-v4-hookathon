#![cfg_attr(target_arch = "riscv32", no_std, no_main)]
include!("data.rs");

extern crate alloc;

use nexus_rt::{read_private_input,write_output, };
use tiny_keccak::{Hasher, Sha3};
use alloc::vec::Vec;

include!("../../common.rs"); // Include the common module

#[nexus_rt::main]
pub fn main() {
    
    let (n_inv_srt,n1_inv) = read_private_input::<(NumberBytes,NumberBytes)>().expect("Failed to read from input");
    
    let (s2_bytes, n_bytes, digest) = tick_volatility2(n_inv_srt, n1_inv);

    // Commit to the public values of the program.
    write_output(&n_inv_srt);
    write_output(&n1_inv);
    write_output(&s2_bytes);
    write_output(&n_bytes);
    write_output(&digest);
    
}

pub fn tick_volatility2(
    n_inv_sqrt: NumberBytes,
    n1_inv: NumberBytes,
) -> (NumberBytes, NumberBytes, [u8; 32]) {
   
    let n = Fixed::from_num(DATA.len());
    let n_inv_sqrt = to_fixed(n_inv_sqrt);
    let n1_inv = to_fixed(n1_inv);

    let data = DATA.into_iter().map(|x| to_fixed(*x)).collect::<Vec<Fixed>>();

    let s2 = tick_volatility(&data, n_inv_sqrt, n1_inv);

    let s2_bytes = to_bytes(s2);
    let n_bytes = to_bytes(n);

    let mut sha3 = Sha3::v256();
    let mut output = [0u8; 32];
    DATA.iter().for_each(|x| sha3.update(x));
    sha3.finalize(&mut output);

    (s2_bytes, n_bytes, output)
}


