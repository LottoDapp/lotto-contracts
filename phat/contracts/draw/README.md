# VrfOracle

The Phat Contract `VrfOracle`, deployed on Phala Network (or testnet):
1) Listens the requests from the Smart Contract, deployed on Astar Network (or testnet)
2) Generates a random value between min and max values
3) Sends the value to the Smart Contract, deployed on Astar Network (or testnet)


## Build

To build the contract:

```bash
cargo contract build
```

## Run Unit tests

To run the unit test:

```bash
cargo test
```

## Run Integration tests

### Deploy the ink! smart contract `vrf_consumer`

Before you can run the tests, you need to have an ink! smart contract deployed in a Substrate node with pallet-contracts.

#### Use the default Ink! smart contract

You can use the default smart contract deployed on Shibuya (`WJFx4kaW59yMD4rpQQbWnUErKr35fo4aEEM7HuukkJkbq7a`).

#### Or deploy your own ink! smart contract

You can build the smart contract
```bash
cd ../../ink/contracts/vrf_consumer
cargo contract build
```
And use Contracts-UI or Polkadot.js to deploy your contract and interact with it.
You will have to configure `alice` or another address as attestor.

### Push some requests

Use Contracts-UI or Polkadot.js to interact with your smart contract deployed on local node or Shibuya and request a random number.


### Run the integration tests

Copy `.env_local` or `.env_shibuya` as `.env` if you haven't done it before. 
It tells the Phat Contract to connect to the Ink! contracts deployed on your local Substrate node or on Shibuya node.

Finally, execute the following command to start integration tests execution.

```bash
cargo test  -- --ignored --test-threads=1
```

### Parallel in Integration Tests

The flag `--test-threads=1` is necessary because by default [Rust unit tests run in parallel](https://doc.rust-lang.org/book/ch11-02-running-tests.html).
There may have a few tests trying to send out transactions at the same time, resulting
conflicting nonce values.
The solution is to add `--test-threads=1`. So the unit test framework knows that you don't want
parallel execution.

### Enable Meta-Tx

Meta transaction allows the Phat Contract to submit rollup tx with attest key signature while using
arbitrary account to pay the gas fee. To enable meta tx in the unit test, change the `.env` file
and specify `SENDER_KEY`.

