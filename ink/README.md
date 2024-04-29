# Ink! Smart Contract

This smart contract manages the states of the lottery.

When the smart contract is instantiated, the state is `NotStarted` and the `lotto manager` can configure the lottery.
Based on the configuration, the players will choose 1, 2, 3, n numbers between `min_number` and `max_number`.

Then, the `lotto manager` starts the lottery with the `start_raffle` function.

When the lottery is started, the participants interact with this contract to register chosen numbers via the `participate` method.

Later, the `lotto manager` completes the lottery with the `complete_raffle` method.
During this operation, a `DrawNumbers` request is sent to the messsage queue. This message is waiting to be proceed by the phat contract.
We use the `phat-offchain-rollup` sdk to manage the communication between ink! smart contract and phat contract: https://github.com/Phala-Network/phat-offchain-rollup/

Afterward, the phat contract sends the winning numbers and the smart contract saves them on the blockchain.
A new `CheckNumber` request is sent to the message queue. This message is waiting to be proceed by the phat contract.

Next, the phat contract sends the winners (or an empty list if there is no winner) and the smart contract save them on the blockchain.
A new lottery can start. Each lottery is identified by an identifier: `raffle_id`.


### Build the contract

```bash
cd contracts/lotto
cargo contract build
```

## Run e2e tests

Before you can run the test, you have to install a Substrate node with pallet-contracts. By default, e2e tests require that you install substrate-contracts-node. You do not need to run it in the background since the node is started for each test independently. To install the latest version:

```bash
cargo install contracts-node --git https://github.com/paritytech/substrate-contracts-node.git
```

If you want to run any other node with pallet-contracts you need to change CONTRACTS_NODE environment variable:

```bash
export CONTRACTS_NODE="YOUR_CONTRACTS_NODE_PATH"
```

And finally execute the following command to start e2e tests execution.

```bash
cd integration_tests
cargo test --features e2e-tests
```
