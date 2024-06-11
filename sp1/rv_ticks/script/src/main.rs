//! A simple script to generate and verify the proof of a given program.

use alloy_sol_types::{sol, SolType};
use clap::Parser;
use fixed::types::I15F17 as Fixed;
use sp1_sdk::{HashableKey, ProverClient, SP1Stdin};
use std::io::{self, BufRead};
use std::time::Instant;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

const ELF: &[u8] = include_bytes!("../../program/elf/riscv32im-succinct-zkvm-elf");

/// The public values encoded as a tuple that can be easily deserialized inside Solidity.
type PublicValuesTuple = sol! {
    tuple( bytes, uint32, uint32, int32, uint32)
};

/// A fixture that can be used to test the verification of SP1 zkVM proofs inside Solidity.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Sp1RvTicksFixture {
    s2: i32,
    n: u32,
    n_inv_sqrt: u32,
    n1_inv: u32,
    vkey: String,
    public_values: String,
    proof: String,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The input CSV file (use '-' for stdin)
    #[arg(short, long)]
    input: String,
}
fn main() {
    let args = Args::parse();
    let ticks: Vec<[u8; 4]> = if args.input == "-" {
        // Read from stdin
        let stdin = io::stdin();
        let mut handle = stdin.lock();
        read_ticks_from_reader(&mut handle)
    } else {
        // Read from file
        let file = std::fs::File::open(args.input).expect("Could not open file");
        let mut reader = std::io::BufReader::new(file);
        read_ticks_from_reader(&mut reader)
    };
    println!("ticks: {:?}", ticks);
    // Calculate  1/(n-1) and the square root of 1/n.
    // These values are used in the volatility proof.
    let n = Fixed::from_num(ticks.len());
    let n_inv_sqrt = Fixed::ONE / n.sqrt();
    let n_inv_sqrt_bytes = Fixed::to_be_bytes(n_inv_sqrt);
    let n1_inv = Fixed::ONE / (n - Fixed::ONE);
    let n1_inv_bytes = Fixed::to_be_bytes(n1_inv);
    // Calculate the volatility squared, s2, using ticks
    // let mut sum_u = Fixed::ZERO;
    // let mut sum_u2 = Fixed::ZERO;
    let mut ticks_prev = Fixed::from_be_bytes(ticks[0]);
    /* for idx in (1..ticks.len()) {
        let ticks_curr = Fixed::from_be_bytes(ticks[idx]);
        let delta = ticks_curr - ticks_prev;
        ticks_prev = ticks_curr;
        sum_u += delta * n_inv_sqrt;
        sum_u2 += delta * delta * n1_inv;
    } */
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

    // setup the inputs;
    let mut stdin = SP1Stdin::new();
    stdin.write(&ticks);
    stdin.write(&n_inv_sqrt_bytes);
    stdin.write(&n1_inv_bytes);

    println!("Configuring new client...");
    let client = ProverClient::new();
    println!("Done.");

    println!("Proving...");
    let (pk, vk) = client.setup(ELF);
    let start_time = Instant::now();

    // Generate proof.
    // let mut proof = client.prove(&pk, stdin).expect("proving failed");
    let mut proof = client.prove_plonk(&pk, stdin).expect("proving failed");
    println!("Done!");
    let prove_time = Instant::now() - start_time;
    println!("Prove time: {} seconds", prove_time.as_secs());

    // Deserialize the public values.
    let bytes = proof.public_values.as_slice();
    let (values, n_inv_sqrt, n1_inv, s2, n) = PublicValuesTuple::abi_decode(bytes, false).unwrap();

    // Create the testing fixture so we can test things end-ot-end.
    let fixture = Sp1RvTicksFixture {
        n_inv_sqrt, 
        n1_inv , 
        s2,
        n,
        vkey: vk.bytes32().to_string(),
        public_values: proof.public_values.bytes().to_string(),
        proof: proof.bytes().to_string(),
    };

    // // Read output.
    // let b = proof.public_values.read::<[u8; 4]>();
    // println!("a: {:?}", a);
    // println!("b: {:?}", b);

    // Verify proof.
    println!("Verifying...");
    //client.verify(&proof, &vk).expect("verification failed");
    client.verify_plonk(&proof, &vk).expect("verification failed");
    println!("Done!");

    // Save proof.
    proof
        .save("proof-with-io.json")
        .expect("saving proof failed");

        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        std::fs::create_dir_all(&fixture_path).expect("failed to create fixture path");
        std::fs::write(
            fixture_path.join("fixture.json"),
            serde_json::to_string_pretty(&fixture).unwrap(),
        )
        .expect("failed to write fixture");


    println!("successfully generated and verified proof for the program!")
}

fn read_ticks_from_reader<R: BufRead>(reader: &mut R) -> Vec<[u8; 4]> {
    let mut ticks = Vec::new();
    let mut line = String::new();
    // Skip the header line
    reader.read_line(&mut line).expect("Failed to read line");
    line.clear();
    while reader.read_line(&mut line).expect("Failed to read line") > 0 {
        if let Ok(value) = line.trim().parse::<i32>() {
            ticks.push(value.to_be_bytes());
        } else {
            panic!("Invalid number in CSV");
        }
        line.clear();
    }
    ticks
}
