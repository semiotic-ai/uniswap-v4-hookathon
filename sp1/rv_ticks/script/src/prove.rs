//! A simple script to generate and verify the proof of a given program.

use crate::build_elf::{self, NumberBytes};
use crate::prove;
use alloy_sol_types::{sol, SolType};
use anyhow::Result;
use fixed::types::I24F40 as Fixed;
use serde::{Deserialize, Serialize};
use sp1_sdk::{HashableKey, ProverClient, SP1Stdin};
use std::fs::read;
use std::path::PathBuf;
use std::time::Instant;
use alloy_network::EthereumWallet;
use alloy_primitives::{address, Bytes, U256, FixedBytes};
use alloy_provider::ProviderBuilder;
use alloy_signer_local::PrivateKeySigner;
use std::env;
use std::str::FromStr;

/// The public values encoded as a tuple that can be easily deserialized inside Solidity.
pub type PublicValuesTuple = sol! {
    tuple( bytes8, bytes8, bytes8, bytes8, bytes32)
};

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
#[derive(Clone)]
pub struct PublicData {
    pub n_inv_sqrt: Fixed,
    pub n1_inv: Fixed,
    pub s2: Fixed,
}


sol! {
    #[sol(rpc)] // <-- Important! Generates the necessary `SnarkBasedFeeOracle` struct and function methods.
    contract SnarkBasedFeeOracle {
        constructor(address) {} // The `deploy` method will also include any constructor arguments.

        #[derive(Debug)]
        function verifyAndUpdate(uint256 claimed_s, bytes proof, bytes public_values);

        #[derive(Debug)]
        function verifyRvProof(bytes proof, bytes public_values) public view returns (bytes8, bytes8, bytes8, bytes8, bytes32);

        #[derive(Debug)]
        function setProgramKey(bytes32 vkey);
    }
}

pub fn setup(elf_path: &str, ticks: Vec<NumberBytes>) -> Result<(Vec<u8>, SP1Stdin, ProverClient)> {
    build_elf::build_elf(ticks.clone(), "src/data.rs", "../program")?;
    let elf = read(elf_path)?;

    let public_io = prove::calculate_public_data(&ticks);
    let stdin = prove::configure_stdin(public_io.clone());
    let client = ProverClient::new();
    Ok((elf, stdin, client))
}

pub fn calculate_public_data(ticks: &[NumberBytes]) -> PublicData {
    let n = Fixed::from_num(ticks.len());
    let n_inv_sqrt = Fixed::ONE / n.sqrt();
    let n1_inv = Fixed::ONE / (n - Fixed::ONE);
    let mut ticks_prev = Fixed::from_num(i64::from_be_bytes(ticks[0]));
    let (sum_u, sum_u2) =
        ticks
            .iter()
            .skip(1)
            .fold((Fixed::ZERO, Fixed::ZERO), |(su, su2), tick| {
                let ticks_curr = Fixed::from_num(i64::from_be_bytes(*tick));
                let delta = ticks_curr - ticks_prev;
                ticks_prev = ticks_curr;
                (su + delta * n_inv_sqrt, su2 + delta * delta * n1_inv)
            });
    let s2 = sum_u2 - (sum_u * sum_u) * n1_inv;
    println!("Volatility squared {}", s2);
    PublicData {
        n_inv_sqrt,
        n1_inv,
        s2,
    }
}
pub fn configure_stdin(public_io: PublicData) -> SP1Stdin {
    let n_inv_sqrt_bytes = Fixed::to_be_bytes(public_io.n_inv_sqrt);
    let n1_inv_bytes = Fixed::to_be_bytes(public_io.n1_inv);
    let mut stdin = SP1Stdin::new();
    stdin.write(&n_inv_sqrt_bytes);
    stdin.write(&n1_inv_bytes);
    stdin
}
async fn send_proof(vkey: FixedBytes<32>, claimed_s: U256, proof: Bytes, public_values: Bytes) -> Result<()> {
    // Need a private key for signing the transaction
    let private_key = env::var("PRIVATE_KEY")?;
    let drpc_key = env::var("DRPC_KEY")?;
    let drpc_url = format!(
        "https://lb.drpc.org/ogrpc?network=sepolia&dkey={}",
        drpc_key
    );
    let signer = PrivateKeySigner::from_bytes(&FixedBytes::from_str(&private_key)?)?;
    let wallet = EthereumWallet::new(signer);

    // Build a provider.
    let provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .wallet(wallet)
        .on_builtin(&drpc_url)
        .await?;

    // Create a new contract instance can be created with `SnarkBasedFeeOracle::new`.
    let address = address!("549225d8eacF9Ee9f0C8F0f0CA1Fde9853245022");
    let contract = SnarkBasedFeeOracle::new(address, &provider);

    let set_program_key_builder = contract.setProgramKey(vkey);
    let set_program_key_return = set_program_key_builder.call().await?;
    println!("{set_program_key_return:?}"); // setProgramKeyReturn
    let _pending_tx = set_program_key_builder.send().await?;

    // Build a call to the `verifyAndUpdate` function and configure it.
    let call_builder = contract.verifyAndUpdate(claimed_s, proof, public_values);

    // Send the call. Note that this is not broadcasted as a transaction.
    let call_return = call_builder.call().await?;
    println!("{call_return:?}"); // verifyAndUpdateReturn

    // Use `send` to broadcast the call as a transaction.
    let _pending_tx = call_builder.send().await?;
    Ok(())
}

pub async fn prove(elf: &[u8], stdin: SP1Stdin, client: ProverClient, push_flag: bool) -> Result<()> {
    // Calculate  1/(n-1) and the square root of 1/n.
    // These values are used in the volatility proof.
    let (pk, vk) = client.setup(elf);

    // Generate proof.
    // let mut proof = client.prove(&pk, stdin).expect("proving failed");
    println!("Proving...");
    let start_time = Instant::now();
    let mut proof = client.prove_plonk(&pk, stdin)?;
    println!("Done!");
    let prove_time = Instant::now() - start_time;
    println!("Prove time: {} seconds", prove_time.as_secs());

    // Read output.
    let s2 = proof.public_values.read::<NumberBytes>();
    let n = proof.public_values.read::<NumberBytes>();
    let digest = proof.public_values.read::<[u8; 32]>();

    // Save proof.
    proof.save("proof-with-io.json")?;

    // Deserialize the public values
    let bytes = proof.public_values.as_slice();
    let (n_inv_sqrt, n1_inv, s2, n, digest) = PublicValuesTuple::abi_decode(bytes, false)?;
    let s2_bytes: NumberBytes = s2.as_slice().try_into()?;
    let n_inv_sqrt_bytes: NumberBytes = n_inv_sqrt.as_slice().try_into()?;
    let n_bytes: NumberBytes = n.as_slice().try_into()?;
    let n1_inv_bytes: NumberBytes = n1_inv.as_slice().try_into()?;
    let s2_fixed = Fixed::from_be_bytes(s2_bytes);
    let s = s2_fixed.sqrt();
    // Create the testing fixture so we can test things end-ot-end.
    let fixture = Sp1RvTicksFixture {
        n_inv_sqrt: u64::from_be_bytes(n_inv_sqrt_bytes),
        n1_inv: u64::from_be_bytes(n1_inv_bytes),
        s: i64::from_be_bytes(s.to_be_bytes()),
        s2: i64::from_be_bytes(s2_bytes),
        n: u64::from_be_bytes(n_bytes),
        digest: digest.to_string(),
        vkey: vk.bytes32().to_string(),
        public_values: proof.public_values.bytes().to_string(),
        proof: proof.bytes().to_string(),
    };

    // Verify proof.
    println!("Verifying...");
    client.verify_plonk(&proof, &vk)?;
    println!("Done!");

    let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    std::fs::create_dir_all(&fixture_path).expect("failed to create fixture path");
    std::fs::write(
        fixture_path.join("fixture.json"),
        serde_json::to_string_pretty(&fixture).unwrap(),
    )?;

    println!("successfully generated and verified proof for the program!");

    if push_flag {
        let vkey_bytes = FixedBytes::<32>::from_str(&vk.bytes32())?;
        let claimed_s = U256::from_be_bytes(s.to_be_bytes());
        let public_values_bytes = Bytes::from_str(&proof.public_values.bytes().to_string())?;
        let proof_bytes = Bytes::from_str(&proof.bytes().to_string())?;
        send_proof(vkey_bytes, claimed_s, proof_bytes, public_values_bytes).await?;
    }
    Ok(())
}

pub fn exec(elf: &[u8], stdin: SP1Stdin, client: ProverClient) -> Result<()> {
    println!("Execution only.");
    let (mut public_values, _) = client.execute(elf, stdin)?;

    // Read output.
    let s2 = public_values.read::<NumberBytes>();
    let n = public_values.read::<NumberBytes>();
    let digest = public_values.read::<[u8; 32]>();

    // Deserialize the public values
    let bytes = public_values.as_slice();
    let (n_inv_sqrt, n1_inv, s2, n, digest) = PublicValuesTuple::abi_decode(bytes, false)?;
    let s2_fixed = Fixed::from_be_bytes(s2.as_slice().try_into()?);
    println!("Volatility squared: {}", s2_fixed);
    let s = s2_fixed.sqrt();
    // Create the testing fixture so we can test things end-ot-end.

    println!("Volatility: {}", s);

    Ok(())
}

#[cfg(test)]

mod tests {
    use super::*;

    #[tokio::test]
    async fn test_send_proof() {
        let fixture_json = include_str!("./fixture.json");
        let fixture: Sp1RvTicksFixture = serde_json::from_str(fixture_json).unwrap();
        let vkey_bytes = FixedBytes::<32>::from_str(&fixture.vkey).unwrap();
        let claimed_s = U256::from(u64::from_be_bytes(fixture.s.to_be_bytes()));
        let public_values_bytes = Bytes::from_str(&fixture.public_values).unwrap();
        let proof_bytes = Bytes::from_str(&fixture.proof).unwrap();
        send_proof(vkey_bytes, claimed_s, proof_bytes, public_values_bytes).await.unwrap();
    }
}
