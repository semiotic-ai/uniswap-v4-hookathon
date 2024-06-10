# Realized volatility against price

This folder contains code that generates a proof of a RISC-V program and verify the proof onchain.

The onchain verification can be done with the [SP1Verifier contract](https://github.com/succinctlabs/sp1-contracts).

Running the script to generate the proof: `RUST_LOG=info cargo run  --release`

It is possible to use the arg `FRI_QUERIES=1` to make the prover use less bits of security, making proof generation faster
**for development purposes only**.

A fixture will be saved into a file called `fixture.json`. This file can be used in foundry, to load the proof
into the smart contract for onchain verification. 

