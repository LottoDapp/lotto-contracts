# Lotto

Lotto is a lottery in which participants choose numbers and if their numbers match with the winning numbers, they win the jackpot.

Data such as "Numbers choose by the participants", "winning numbers" and "the winners" are registered in the blockchain by the Ink! smart contract. 

A phat contract is on charge to randomly draw the numbers via a `verifiable random function` and search the potential winners via a query on the indexer. 

A lottery has different states:
 - `NotStarted`: no lottery is started and no participant can choose numbers.
 - `Ongoing`: the lottery is started and the participants can start to play,
 - `WaitingResults`: the participants can not play anymore and we are waiting for the winning numbers.
 - `WaitingWinners`: the winning numbers are saved on the blockchain and we are waiting for potential winner(s).
 - `Closed`: the lottery is closed, the potential winners are saved on the blockchain. A new lottery can start.

## Prerequisites

Rust 1.76 and cargo-contract 3.2 must be installed to compile the smart and phat contract.s

## Smart contract 

`lotto_contract` is an Ink! smart contract deployed on Shibuya/Astar Network.
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


## Phat contract

The phat contract is an offchain rollup in charge to proceed the messages sent by the ink! smart contract: 
- when a `DrawNumbers` request is sent by the smart contract, the phat contract uses the `pink_extension::vrf` to randomly provide the winning numbers.
- when a `CheckWinners` request is sent by the smart contract, the phat contract reads the SubQuery indexer to check the winners and send them to ink! smart contract.
You can find more information about the communication between ink! smart contract and phat contract [here](https://github.com/Phala-Network/phat-offchain-rollup/).