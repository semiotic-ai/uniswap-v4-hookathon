//! A simple script to generate and verify the proof of a given program.

use alloy_sol_types::{sol, SolType};
use clap::Parser;
use fixed::types::I15F17 as Fixed;
use sp1_sdk::{HashableKey, ProverClient, SP1Stdin};
use std::time::Instant;
use std::path:: PathBuf;
use serde::{Deserialize, Serialize};
include!("../../program/src/data.rs");

const ELF: &[u8] = include_bytes!("../../program/elf/riscv32im-succinct-zkvm-elf");
/// The public values encoded as a tuple that can be easily deserialized inside Solidity.
type PublicValuesTuple = sol! {
    tuple( bytes4, bytes4, bytes4, bytes4, bytes32)
};

/// A fixture that can be used to test the verification of SP1 zkVM proofs inside Solidity.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Sp1RvTicksFixture {
    s2: i32,
    n: u32,
    n_inv_sqrt: u32,
    n1_inv: u32,
    digest: String,
    vkey: String,
    public_values: String,
    proof: String,
}
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// A flag to enable proof generation. Otherwise, the RISC-V program is executed and the public
    /// values are returned.
    #[arg(short, long)]
    prove: bool,
}

fn main() {
    let args = Args::parse();
    let build_proof = args.prove;
    println!("Build proof: {}", build_proof);

    let ticks = DATA;

    // Calculate  1/(n-1) and the square root of 1/n.
    // These values are used in the volatility proof.
    let n = Fixed::from_num(ticks.len());
    let n_inv_sqrt = Fixed::ONE / n.sqrt();
    let n_inv_sqrt_bytes = Fixed::to_be_bytes(n_inv_sqrt);
    let n1_inv = Fixed::ONE / (n - Fixed::ONE);
    let n1_inv_bytes = Fixed::to_be_bytes(n1_inv);
    let mut ticks_prev = Fixed::from_be_bytes(ticks[0]);
    let (sum_u, sum_u2) =
        ticks
            .iter()
            .skip(1)
            .fold((Fixed::ZERO, Fixed::ZERO), |(su, su2), tick| {
                let ticks_curr = Fixed::from_be_bytes(*tick);
                let delta = ticks_curr - ticks_prev;
                ticks_prev = ticks_curr;
                (su + delta * n_inv_sqrt, su2 + delta * delta * n1_inv)
            });
   
    let s2 = sum_u2 - (sum_u * sum_u) * n1_inv;
    println!("Volatility squared: {}", s2);
    println!("... as bytes: {:?}", Fixed::to_be_bytes(s2));

    let s = s2.sqrt();
    println!("Volatility: {}", s);

    let s2_bytes = Fixed::to_be_bytes(s2);
    let s2_int32 = i32::from_be_bytes(s2_bytes); 
    println!("Volatility squared, i32: {}", s2_int32);

    let s_int32 = i32::from_be_bytes(s.to_be_bytes());
    println!("Volatility, i32: {}", s_int32);
    
    // setup the inputs;
    let mut stdin = SP1Stdin::new();
    stdin.write(&n_inv_sqrt_bytes);
    stdin.write(&n1_inv_bytes);

    println!("Configuring new client...");
    let client = ProverClient::new();
    println!("Done.");

    let (pk, vk) = client.setup(ELF);

    if build_proof {
        // Generate proof.
        // let mut proof = client.prove(&pk, stdin).expect("proving failed");
        println!("Proving...");
        let start_time = Instant::now();
        let mut proof = client.prove_plonk(&pk, stdin).expect("proving failed");
        println!("Done!");
        let prove_time = Instant::now() - start_time;
        println!("Prove time: {} seconds", prove_time.as_secs());

        // Read output.
        let s2 = proof.public_values.read::<[u8; 4]>();
        let n = proof.public_values.read::<[u8; 4]>();
        let digest = proof.public_values.read::<[u8; 32]>();
        println!("s2: {:?}", s2);
        println!("n: {:?}", n);
        println!("digest: {:?}", digest);
        
        // Save proof.
        proof
            .save("proof-with-io.json")
            .expect("saving proof failed");
       
        // Deserialize the public values
        let bytes = proof.public_values.as_slice();
        let (n_inv_sqrt, n1_inv, s2, n, digest) = PublicValuesTuple::abi_decode(bytes, false).unwrap();

        // Create the testing fixture so we can test things end-ot-end.
        let fixture = Sp1RvTicksFixture {
            n_inv_sqrt: n_inv_sqrt.into(), 
            n1_inv: n1_inv.into(), 
            s2: s2.into(),
            n: n.into(),
            digest: digest.to_string(),
            vkey: vk.bytes32().to_string(),
            public_values: proof.public_values.bytes().to_string(),
            proof: proof.bytes().to_string(),
        };


        // Verify proof.
        println!("Verifying...");
        //client.verify(&proof, &vk).expect("verification failed");
        client.verify_plonk(&proof, &vk).expect("verification failed");
        println!("Done!");


        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        std::fs::create_dir_all(&fixture_path).expect("failed to create fixture path");
        std::fs::write(
            fixture_path.join("fixture.json"),
            serde_json::to_string_pretty(&fixture).unwrap(),
            )
            .expect("failed to write fixture");


        println!("successfully generated and verified proof for the program!")

    } else {
        // Only execute the program and get a `SP1PublicValues` object.
        println!("Executing RISC-V program...");
        let client = ProverClient::new();
        let ( public_values, _) = client.execute(ELF, stdin).unwrap();
        
        // Read output.
        let bytes = public_values.as_slice();
        let (_n_inv_sqrt, _n1_inv, s2, n, digest) = PublicValuesTuple::abi_decode(bytes, false).unwrap();
        
        println!("s2_bytes: {:?}", s2.as_slice());
        println!("n: {}", n);
        println!("digest: {}", digest);

        
        let s2_int32 = i32::from_be_bytes(s2.as_slice().try_into().expect("Invalid bytes"));
        let s2_fixed = Fixed::from_be_bytes(s2.as_slice().try_into().expect("Invalid bytes"));
        
        println!("Volatility squared: {}", s2_fixed);

        let s = s2_fixed.sqrt();
        println!("Volatility: {}", s);

        println!("Volatility squared, i32 {}", s2_int32);

        let s_int32 = i32::from_be_bytes(s.to_be_bytes());
        println!("Volatility, i32: {}", s_int32);
        println!("s_int32 * s_int32: {}", (s_int32 * s_int32) >> Fixed::FRAC_NBITS);
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compare_f32_to_fixed() {
        let ticks = DATA;
        
        // We can compare fixed point to floating point results
        let mut ticks_prev = 0.0;
        let n_f32: f32 = ticks.len() as f32;
        let n_inv_sqrt_f32: f32 = 1.0 / n_f32.sqrt();
        let n1_inv_f32: f32 = 1.0 / (n_f32 - 1.0);
        let (sum_u_f32, sum_u2_f32) =
            ticks
                .iter()
                .skip(1)
               .fold((0.0, 0.0), |(su, su2), tick| {
                    let ticks_curr = i32::from_be_bytes(*tick) as f32;
                    let delta = ticks_curr - ticks_prev;
                    ticks_prev = ticks_curr;
                    (su + delta * n_inv_sqrt_f32, su2 + delta * delta * n1_inv_f32)
                });

        // s2 = s * s
        //    = s_int * SCALE_FACTOR * s_int * SCALE_FACTOR 
        let scale_factor = 0.5_f32.powf(Fixed::FRAC_NBITS as f32);
        let s2_f32 = (sum_u2_f32 - (sum_u_f32 * sum_u_f32) * n1_inv_f32) * scale_factor * scale_factor;
        println!("Volatility squared, f32: {}", s2_f32);
        let s_f32 = s2_f32.sqrt();
        println!("Volatility, f32: {}", s_f32);
    
        // Calculate  1/(n-1) and the square root of 1/n.
        // These values are used in the volatility proof.
        let n = Fixed::from_num(ticks.len());
        let n_inv_sqrt = Fixed::ONE / n.sqrt();
        let _n_inv_sqrt_bytes = Fixed::to_be_bytes(n_inv_sqrt);
        let n1_inv = Fixed::ONE / (n - Fixed::ONE);
        let _n1_inv_bytes = Fixed::to_be_bytes(n1_inv);
        let mut ticks_prev = Fixed::from_be_bytes(ticks[0]);
        let (sum_u, sum_u2) =
            ticks
                .iter()
                .skip(1)
                .fold((Fixed::ZERO, Fixed::ZERO), |(su, su2), tick| {
                    let ticks_curr = Fixed::from_be_bytes(*tick);
                    let delta = ticks_curr - ticks_prev;
                    ticks_prev = ticks_curr;
                    (su + delta * n_inv_sqrt, su2 + delta * delta * n1_inv)
                });
       
        let s2 = sum_u2 - (sum_u * sum_u) * n1_inv;
        println!("Volatility squared: {}", s2);
        println!("... as bytes: {:?}", Fixed::to_be_bytes(s2));

        let s = s2.sqrt();
        println!("Volatility: {}", s);

        let s2_bytes = Fixed::to_be_bytes(s2);
        let s2_int32 = i32::from_be_bytes(s2_bytes); 
        println!("Volatility squared, i32: {}", s2_int32);

        let s_int32 = i32::from_be_bytes(s.to_be_bytes());
        println!("Volatility, i32: {}", s_int32);
        // We can do arithmetic with the integer representation of the fixed point number.
        // We just need to properly account for the scale factor and ensure that we don't overflow.
        // See how the scale factor manifests below:
        // s2 = s2_int / SCALE_FACTOR
        // s = s_int / SCALE_FACTOR
        // s * s = (s_int / SCALE_FACTOR) * (s_int / SCALE_FACTOR)
        //       = s2_int / SCALE_FACTOR
        // => s2_int = s_int * s_int * SCALE_FACTOR (need to be aware of overflow when multiplying
        // s_int)
        
        let s2_with_error = (s_int32 as i64 * s_int32 as i64) >> Fixed::FRAC_NBITS;
        println!("s_int32 * s_int32: {}", s2_with_error); 
        // s2.sqrt() has error <= DELTA = 1/2^(FRAC_NBITS)
        // So s2.sqrt() * s2.sqrt() has error <= 2*S*DELTA + DELTA^2
        assert!((s2_with_error - s2_int32 as i64).abs() <= 2 * s_int32 as i64 + 1);
        println!("error: {}", (s2_with_error - s2_int32 as i64).abs());
        println!("Expected error: {}", 2 * s_int32 as i64 + 1);
    }

}
