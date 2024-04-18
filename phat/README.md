# Lucky Phat Contracts

## dAppStaking

The `dapp_staking` calls the `dAppStaking` pallet to claim the dApp rewards.

More information [here](contracts/dapp_staking)

## Raffle
 
Phat contract that manages the raffle.
The `raffle` phat contract:
- reads the data from `raffle_consumer` contract, 
- via a js script, queries the indexer to get the participants and runs the raffle,
- sends the output to `raffle_consumer` contract.

More information [here](contracts/raffle)
