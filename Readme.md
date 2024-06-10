# uniswap v4 hookathon

This solution gives the realized volatility to a smart contract, without relying on Oracles, by using SNARKs with SP1.

The workflow goes as follows:

1. Data for calculating realized volatility is parsed using a [substream](https://substreams.streamingfast.io/). This data in specific is composed by events from a `UniswapV3Pool` contract. In this contract, we are particularly interested in the `ticks` or `sqrtPriceX96` values. For more details, check the `realized_volatility_substream` folder.

2. This data is fed into [sp1 program](https://succinctlabs.github.io/sp1/writing-programs/basics.html) to generate a proof that the realized volatility calculation
was done correctly. Check how it is done is in `sp1` folder.

3. Then, the proof is verified onchain, via a `SP1Verifier` smart contract. A proof is written in `.json` and can be generated from the `sp1` folder script. Check the `VolatilityHook-UniV4` for the contracts tests and integrations.

