# Uniswap V3 pool tick volatility calculation with Axiom proofs
We tried to demonstrate fast volatility calculation with axiom. the calculation is fully
optimized via axiom's circuit implementation to be able to process more data. Also some 
parts of the fixed point implementation is used for better precision.

We use a sample CSV file (see data/ticks_8192.csv) to verify with reference implementation
in SP1 use case.

## Requirement 
- Rust 1.7+

## Setup

```sh 
EXPORT PROVIDER_URI=<ETH_NODE_RPC_ENDPOINT>
cargo run --release -- --input data/inputs.json --config data/config.json --degree 15 keygen
```

## Proof Generation
After generating related keys you can update the source file and create proofs 
```sh 
EXPORT PROVIDER_URI=<ETH_NODE_RPC_ENDPOINT>
cargo run --release -- --input data/inputs.json --config data/config.json --degree 15 prove
```

**Reference** calculation is the code used in SP1.

**Optimized** calculation removes redundant steps and minimizes divisions for better fixed point results.

**Axiom** uses the optimized calculation with proof.

Our experience on a 16x2.7Ghz is around 2.2sec to calculate proof for 8192 tick samples.