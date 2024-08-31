# Volatility Calculation for Uniswap V3 Pool Ticks Using Axiom Proofs
We demonstrate a fast and efficient method for calculating volatility using Axiom. 
This approach is fully optimized through Axiom's circuit implementation, allowing for 
the processing of larger datasets. Additionally, parts of the fixed-point implementation
 have been utilized to enhance precision.

To validate the results, we use a sample CSV file (ticks_8192.csv) and compare it against a reference implementation in the SP1 use case.

- **Reference Value:** The code used in SP1.
- **Optimized Value:** This version eliminates redundant steps and reduces the number of divisions to improve fixed-point accuracy.
- **Axiom Value :** Applies the optimized calculation along with proof generation. We expect to have some difference from `Optimized`version due to fixed point implementation.

## Requirement 
- Rust 1.7+

## Testing
If needed update `data/inputs.json` with new data set. !!! Also update the related `SAMPLE_SIZE` constant in `src/main.rs, as we also generating witness for inputs ans axiom needs know the exact input size.

Then generate related keys

```sh 
EXPORT PROVIDER_URI=<ETH_NODE_RPC_ENDPOINT>
cargo run --release -- --input data/inputs.json --config data/config.json --degree 15 keygen
```

and `run` to have the snark proof and output

```sh 
EXPORT PROVIDER_URI=<ETH_NODE_RPC_ENDPOINT>
cargo run --release -- --input data/inputs.json --config data/config.json --degree 15 run
```

On a 16-core, 2.7 GHz processor, proof generation for 8,192 tick samples takes approximately 2 seconds.