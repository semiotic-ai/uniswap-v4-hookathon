//! A simple script to generate and verify the proof of a given program.

use alloy_sol_types::{sol, SolType};
use clap::Parser;
use fixed::types::I24F40 as Fixed;
use sp1_sdk::ProverClient;
use serde::{Deserialize, Serialize};
use std::fs::read;

mod prove;
mod build_elf;

use build_elf::{TickSource, build_elf};

const ELF_PATH: &str = "../program/elf/riscv32im-succinct-zkvm-elf";
/// The public values encoded as a tuple that can be easily deserialized inside Solidity.
type PublicValuesTuple = sol! {
    tuple( bytes8, bytes8, bytes8, bytes8, bytes32)
};

type NumberBytes = [u8; 8];
/// A fixture that can be used to test the verification of SP1 zkVM proofs inside Solidity.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Sp1RvTicksFixture {
    s: i64,
    s2: i64,
    n: u64,
    n_inv_sqrt: u64,
    n1_inv: u64,
    digest: String,
    vkey: String,
    public_values: String,
    proof: String,
}
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// A flag to specify ticks TickSource
    #[arg(short, long)]
    ticks: Option<String>
}

fn main() {
    let args = Args::parse();
    let ticks_source = match args.ticks {
        Some(ticks) => TickSource::Jsonl(ticks),
        None => TickSource::Random
    };
    let ticks = build_elf::build_elf(ticks_source, "src/data.rs", "../program").unwrap();
    let elf = read(ELF_PATH).unwrap();

    let public_io = prove::calculate_public_data(&ticks);
    let stdin = prove::configure_stdin(public_io.clone());
    let client = ProverClient::new();
    prove::prove(elf.as_slice(), stdin, client).unwrap();
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exec() {
        // Only execute the program and get a `SP1PublicValues` object.
        println!("Executing RISC-V program...");
        let client = ProverClient::new();
        let ( public_values, _) = client.execute(elf.as_slice(), stdin).unwrap();
        
        // Read output.
        let bytes = public_values.as_slice();
        let (_n_inv_sqrt, _n1_inv, s2, n, digest) = PublicValuesTuple::abi_decode(bytes, false).unwrap();
        
        println!("s2_bytes: {:?}", s2.as_slice());
        println!("s2 i64: {}", i64::from(s2));
        println!("n: {}", n);
        println!("digest: {}", digest);

        let s2_fixed = Fixed::from_be_bytes(s2.as_slice().try_into().unwrap());
        println!("Volatility test: {}", public_io.s2);
        println!("Volatility bytes test: {:?}", Fixed::to_be_bytes(public_io.s2));
        println!("Volatility squared: {}", s2_fixed);

        let s = s2_fixed.sqrt();
        println!("Volatility: {}", s);

        let s_int64 = i64::from_be_bytes(s.to_be_bytes());
        println!("Volatility, i64: {}", s_int64);
    }
    #[test]
    fn test_compare_f64_to_fixed() {
        let ticks = DATA;
        
        // We can compare fixed point to floating point results
        let mut ticks_prev = i64::from_be_bytes(ticks[0]) as f64;
        println!("ticks_prev: {}", ticks_prev);
        let n_f64: f64 = ticks.len() as f64;
        let n_inv_sqrt_f64: f64 = 1.0 / n_f64.sqrt();
        let n1_inv_f64: f64 = 1.0 / (n_f64 - 1.0);
        let (sum_u_f64, sum_u2_f64) =
            ticks
                .iter()
                .skip(1)
               .fold((0.0, 0.0), |(su, su2), tick| {
                    let ticks_curr = i64::from_be_bytes(*tick) as f64;
                    let delta = ticks_curr - ticks_prev;
                    ticks_prev = ticks_curr;
                    (su + delta * n_inv_sqrt_f64, su2 + delta * delta * n1_inv_f64)
                });

        // s2 = s * s
        //    = s_int * SCALE_FACTOR * s_int * SCALE_FACTOR 
        let scale_factor = 1.0;
        let s2_f64 = (sum_u2_f64 - (sum_u_f64 * sum_u_f64) * n1_inv_f64) * scale_factor * scale_factor;
        println!("Volatility squared, f64: {}", s2_f64);
        let s_f64 = s2_f64.sqrt();
        println!("Volatility, f64: {}", s_f64);
        println!("Volatility ln: {}", s_f64 * 1.0001_f64.ln());
    
        // Calculate  1/(n-1) and the square root of 1/n.
        // These values are used in the volatility proof.
        let n = Fixed::from_num(ticks.len());
        let n_inv_sqrt = Fixed::ONE / n.sqrt();
        let _n_inv_sqrt_bytes = Fixed::to_be_bytes(n_inv_sqrt);
        let n1_inv = Fixed::ONE / (n - Fixed::ONE);
        let n_inv = Fixed::ONE / n;
        let _n1_inv_bytes = Fixed::to_be_bytes(n1_inv);
        let mut ticks_prev = Fixed::from_num(u64::from_be_bytes(ticks[0]));
        println!("ticks_prev: {}", ticks_prev);
        let (sum_u, sum_u2) =
            ticks
                .iter()
                .skip(1)
                .fold((Fixed::ZERO, Fixed::ZERO), |(su, su2), tick| {
                    let ticks_curr = Fixed::from_num(u64::from_be_bytes(*tick));
                    let delta = ticks_curr - ticks_prev;
                    ticks_prev = ticks_curr;
                    (su + delta * n_inv_sqrt, su2 + delta * delta  * n1_inv)
                });
       
        let s2 = sum_u2  - (sum_u * sum_u) * n1_inv;
        println!("Volatility squared: {}", s2);
        println!("... as bytes: {:?}", Fixed::to_be_bytes(s2));

        let s = s2.sqrt();
        println!("Volatility: {}", s);

        let s2_bytes = Fixed::to_be_bytes(s2);
        let s2_int32 = i64::from_be_bytes(s2_bytes); 
        println!("Volatility squared, i64: {}", s2_int32);

        let s_int32 = i64::from_be_bytes(s.to_be_bytes());
        println!("Volatility, i64: {}", s_int32);
       
        // We can do arithmetic with the integer representation of the fixed point number.
        // We just need to properly account for the scale factor and ensure that we don't overflow.
        // See how the scale factor manifests below:
        // s2 = s2_int / SCALE_FACTOR
        // s = s_int / SCALE_FACTOR
        // s * s = (s_int / SCALE_FACTOR) * (s_int / SCALE_FACTOR)
        //       = s2_int / SCALE_FACTOR
        // => s2_int = s_int * s_int * SCALE_FACTOR (need to be aware of overflow when multiplying
        // s_int)
        
        let s2_with_error = s_int32 as i64 * (s_int32 as i64 >> Fixed::FRAC_NBITS);
        println!("s_int32 * s_int32: {}", s2_with_error); 
        let s2_with_error_fixed = Fixed::from_num(s2_with_error>> Fixed::FRAC_NBITS);
        println!("s_int32 * s_int32: {}", s2_with_error_fixed);
        // s2.sqrt() has error <= DELTA = 1/2^(FRAC_NBITS)
        // So s2.sqrt() * s2.sqrt() has error <= 2*S*DELTA + DELTA^2
        assert!((s2_with_error - s2_int32 as i64).abs() <= 2 * s_int32 as i64 + 1);
        println!("error: {}", (s2_with_error - s2_int32 as i64).abs());
        println!("Expected error: {}", 2 * s_int32 as i64 + 1);
    }

    #[test]
    fn test_log_bases() {
        let ln1_0001_f64 = 1.0001_f64.ln();
        let ln1_0001 = Fixed::from_num(ln1_0001_f64);
        let ln1_0001_bytes = Fixed::to_be_bytes(ln1_0001);
        let ln1_0001_int32 = i64::from_be_bytes(ln1_0001_bytes);
        println!("ln(1.0001): {}", ln1_0001_int32);
    }

} 
