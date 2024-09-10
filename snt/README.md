# Volatility Calculation for Uniswap V3 Pool Ticks Using Space Time SQL Proof
We tried to implement a volatility calculation via querying with space and time proof sql.
As the framework is very incomplete in SQL functionality, volatility can not be calculated.

```SQL
SELECT STDEV(ticks) FROM table
```

STDEV = √((Σ(t<sub>n</sub> -t<sub>n-1</sub>)<sup>2</sup>)/n)

