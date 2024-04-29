# Phat Contract

The phat contract is an offchain rollup in charge to proceed the messages sent by the ink! smart contract:
- when a `DrawNumbers` request is sent by the smart contract, the phat contract uses the `pink_extension::vrf` to randomly provide the winning numbers.
- when a `CheckWinners` request is sent by the smart contract, the phat contract reads the SubQuery indexer to check the winners and send them to ink! smart contract.
  You can find more information about the communication between ink! smart contract and phat contract [here](https://github.com/Phala-Network/phat-offchain-rollup/).

The Phat Contract `LottoDrow`, deployed on Phala Network (or testnet):
1) Listens the requests from the Smart Contract deployed on Astar Network (or testnet)
2) If a `DrawNumbers` request is sent, the phat contract uses the `pink_extension::vrf` to randomly provide the winning numbers. If a `CheckWinners` request is sent, the phat contract reads the SubQuery indexer to check the winners.
3) Sends the response to the Smart Contract, deployed on Astar Network (or testnet)


## Build

To build the contract:

```bash
cargo contract build
```

## Run Unit tests

Before you can run the tests, you need to configure the phat contract.
Copy `.env_local` or `.env_shibuya` as `.env` if you haven't done it before.

To run the unit test:

```bash
cargo test
```

## Run Integration tests

### Deploy the ink! smart contract `lotto_contract`

Before you can run the tests, you need to have an ink! smart contract deployed in a Substrate node with pallet-contracts.

#### Use the default Ink! smart contract

You can use the default smart contract deployed on Shibuya (`aB9AxBVmoYogZ5ZAX662R5YJafTVCqVbtGzJYX3LvvwZW5r`).

#### Or deploy your own ink! smart contract

You can build the smart contract
```bash
cd ../../ink/contracts/lotto
cargo contract build
```
And use Contracts-UI or Polkadot.js to deploy your contract and interact with it.
You will have to configure `alice` or another address as attestor.

### Push some requests

Use Contracts-UI or Polkadot.js to interact with your smart contract deployed on local node or Shibuya.

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

