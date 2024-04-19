#![cfg_attr(not(feature = "std"), no_std, no_main)]
#[cfg(all(test, feature = "e2e-tests"))]
mod e2e_tests {

    use ink::env::DefaultEnvironment;
    use ink_e2e::subxt::tx::Signer;
    use ink_e2e::{build_message, PolkadotConfig};
    use openbrush::contracts::access_control::accesscontrol_external::AccessControl;
    use openbrush::traits::AccountId;
    use scale::Encode;

    use lotto::traits::Number;
    use lotto::traits::raffle::raffle_external::Raffle;
    use lotto::traits::config::Config;
    use lotto::traits::config::raffleconfig_external::RaffleConfig;

    use lotto_contract::{ lotto_contract, *};

    use phat_rollup_anchor_ink::traits::meta_transaction::metatransaction_external::MetaTransaction;
    use phat_rollup_anchor_ink::traits::rollup_anchor::rollupanchor_external::RollupAnchor;

    use phat_rollup_anchor_ink::traits::{
        rollup_anchor, rollup_anchor::*,
    };

    type E2EResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;


    async fn alice_instantiates_contract(
        client: &mut ink_e2e::Client<PolkadotConfig, DefaultEnvironment>,
    ) -> AccountId {
        let lotto_constructor = lotto_contract::ContractRef::new();
        let lotto_contract_id = client
            .instantiate(
                "lotto_contract",
                &ink_e2e::alice(),
                lotto_constructor,
                0,
                None,
            )
            .await
            .expect("instantiate failed")
            .account_id;

        lotto_contract_id
    }

    async fn alice_configures_contract(
        client: &mut ink_e2e::Client<PolkadotConfig, DefaultEnvironment>,
        contract_id: &AccountId,
    ) {
        let config = Config {
            nb_numbers: 4,
            min_number: 1,
            max_number: 50,
        };
        let set_config = build_message::<lotto_contract::ContractRef>(
            contract_id.clone(),
        )
        .call(|contract| contract.set_config(config));
        client
            .call(&ink_e2e::alice(), set_config, 0, None)
            .await
            .expect("set config failed");
    }

    async fn alice_grants_bob_as_attestor(
        client: &mut ink_e2e::Client<PolkadotConfig, DefaultEnvironment>,
        contract_id: &AccountId,
    ) {
        // bob is granted as attestor
        let bob_address = ink::primitives::AccountId::from(ink_e2e::bob().public_key().0);
        let grant_role = build_message::<lotto_contract::ContractRef>(contract_id.clone())
            .call(|oracle| oracle.grant_role(ATTESTOR_ROLE, Some(bob_address)));
        client
            .call(&ink_e2e::alice(), grant_role, 0, None)
            .await
            .expect("grant bob as attestor failed");
    }

    async fn alice_starts_raffle(
        client: &mut ink_e2e::Client<PolkadotConfig, DefaultEnvironment>,
        contract_id: &AccountId,
        raffle_num: u32,
    ) {
        let start_raffle = build_message::<lotto_contract::ContractRef>(
            contract_id.clone(),
        )
            .call(|contract| contract.start_raffle(raffle_num));
        client
            .call(&ink_e2e::alice(), start_raffle, 0, None)
            .await
            .expect("start raffle failed");
    }

    async fn alice_stops_raffle(
        client: &mut ink_e2e::Client<PolkadotConfig, DefaultEnvironment>,
        contract_id: &AccountId,
    ) {
        let stop_raffle = build_message::<lotto_contract::ContractRef>(
            contract_id.clone(),
        )
            .call(|contract| contract.complete_raffle());
        client
            .call(&ink_e2e::alice(), stop_raffle, 0, None)
            .await
            .expect("stop raffle failed");
    }

    async fn participates(
        client: &mut ink_e2e::Client<PolkadotConfig, DefaultEnvironment>,
        contract_id: &AccountId,
        signer: &ink_e2e::Keypair,
        numbers: Vec<Number>,
    ) {
        let participate = build_message::<lotto_contract::ContractRef>(
            contract_id.clone(),
        )
            .call(|contract| contract.participate(numbers.clone()));
        client
            .call(signer, participate, 0, None)
            .await
            .expect("Participate failed");
    }

    #[ink_e2e::test(
        additional_contracts = "contracts/lotto/Cargo.toml"
    )]
    async fn test_do_raffle(mut client: ink_e2e::Client<C, E>) -> E2EResult<()> {
        // given
        let contract_id = alice_instantiates_contract(&mut client).await;

        // configure the raffle
        alice_configures_contract(&mut client, &contract_id).await;

        // bob is granted as attestor
        alice_grants_bob_as_attestor(&mut client, &contract_id).await;

        let raffle_num = 3;

        // start the raffle
        alice_starts_raffle(&mut client, &contract_id, raffle_num).await;

        // dave participates
        participates(&mut client, &contract_id, &ink_e2e::dave(), vec![5, 40, 8, 2]);
        participates(&mut client, &contract_id, &ink_e2e::dave(), vec![3, 6, 7, 5]);
        participates(&mut client, &contract_id, &ink_e2e::dave(), vec![12, 4, 6, 2]);
        participates(&mut client, &contract_id, &ink_e2e::dave(), vec![15, 44, 4, 1]);
        // same numbers are incorrect but it should not failed
        participates(&mut client, &contract_id, &ink_e2e::dave(), vec![15, 44, 1, 1]);

        // charlie participates
        participates(&mut client, &contract_id, &ink_e2e::charlie(), vec![52, 3, 8, 2]);
        participates(&mut client, &contract_id, &ink_e2e::charlie(), vec![34, 6, 2, 5]);
        participates(&mut client, &contract_id, &ink_e2e::charlie(), vec![12, 4, 6, 4]);


        // stop the raffle
        alice_stops_raffle(&mut client, &contract_id).await;



        let dave_address = ink::primitives::AccountId::from(ink_e2e::dave().public_key().0);
/*
        // data is received
        let response = RaffleResponseMessage {
            era: 13,
            skipped: false,
            rewards: 100,
            winners: [dave_address].to_vec(),
        };

        let payload = JsResponse {
            js_script_hash: [1u8; 32],
            input_hash: [3u8; 32],
            settings_hash: [2u8; 32],
            output_value: response.encode(),
        };
        let actions = vec![HandleActionInput::Reply(payload.encode())];
        let rollup_cond_eq = build_message::<lotto_contract::ContractRef>(contract_id.clone())
            .call(|oracle| oracle.rollup_cond_eq(vec![], vec![], actions.clone()));
*/
        /*
               let result = client
                   .call_dry_run(&ink_e2e::bob(), &rollup_cond_eq, 0, None)
                   .await;
               assert_eq!(result.debug_message(), "e");
        */
/*
        let result = client
            .call(&ink_e2e::bob(), rollup_cond_eq, 0, None)
            .await
            .expect("rollup cond eq should be ok");
        // two events : MessageProcessedTo and RaffleDone
        assert!(result.contains_event("Contracts", "ContractEmitted"));

        // test wrong era => meaning only 1 raffle by era
        let rollup_cond_eq = build_message::<lotto_contract::ContractRef>(contract_id.clone())
            .call(|oracle| oracle.rollup_cond_eq(vec![], vec![], actions.clone()));
        let result = client.call(&ink_e2e::bob(), rollup_cond_eq, 0, None).await;
        assert!(result.is_err(), "Era must be sequential without blank");

        // and check if the data is filled
        let get_next_era = build_message::<lotto_contract::ContractRef>(contract_id.clone())
            .call(|contract| contract.get_next_era());
        let next_era = client
            .call_dry_run(&ink_e2e::charlie(), &get_next_era, 0, None)
            .await
            .return_value()
            .expect("next era failed");

        assert_eq!(14, next_era);

        // check the balance of the developer contract
        let dev_contract_balance = client
            .balance(contracts.dapps_staking_developer_account_id)
            .await
            .expect("getting dev contract balance failed");

        assert_eq!(1000000090, dev_contract_balance);

        // check the balance of the reward manager
        let reward_manager_contract_balance = client
            .balance(contracts.reward_manager_account_id)
            .await
            .expect("getting reward manager contract balance failed");

        assert_eq!(1000000010, reward_manager_contract_balance);

        // check the balance of the raffle contract
        let lotto_contract_balance = client
            .balance(contracts.lotto_contract_account_id)
            .await
            .expect("getting raffle contract balance failed");

        assert_eq!(1000000000, lotto_contract_balance);

        // check the balance of dave
        let dave_balance_before_claim = client
            .balance(dave_address)
            .await
            .expect("getting Dave balance failed");

        let claim =
            build_message::<reward_manager::ContractRef>(contracts.reward_manager_account_id)
                .call(|contract| contract.claim());

        let result = client
            .call(&ink_e2e::dave(), claim, 0, None)
            .await
            .expect("Claim rewards should be ok");
        // 1 event : RewardsClaimed
        assert!(result.contains_event("Contracts", "ContractEmitted"));

        // check the balance of dave
        let dave_balance_after_claim = client
            .balance(dave_address)
            .await
            .expect("getting Dave balance failed");

        // we cannot calculate the balance because of fees
        assert!(dave_balance_after_claim > dave_balance_before_claim);

        // check the balance of the reward manager
        let reward_manager_contract_balance_after_claim = client
            .balance(contracts.reward_manager_account_id)
            .await
            .expect("getting reward manager contract balance failed");

        assert_eq!(
            reward_manager_contract_balance_after_claim,
            reward_manager_contract_balance - 10
        );
 */
        Ok(())
    }


    #[ink_e2e::test(
        additional_contracts = "contracts/lotto/Cargo.toml"
    )]
    async fn test_receive_error(mut client: ink_e2e::Client<C, E>) -> E2EResult<()> {
        // given
        let contract_id = alice_instantiates_contract(&mut client).await;

        // bob is granted as attestor
        alice_grants_bob_as_attestor(&mut client, &contract_id).await;

        let raffle_num = 1;

        let input_data = LottoRequestMessage {
            requestor_id: contract_id.clone(),
            draw_num: raffle_num,
            request: Request::DrawNumbers(4, 1, 50),
        };

        // then a response is received
        /*
        let error = vec![3u8; 5];
        let payload = rollup_anchor::ResponseMessage::Error {
            input_value: input_data.encode(),
            error,
        };
        let actions = vec![HandleActionInput::Reply(payload.encode())];
        let rollup_cond_eq = build_message::<lotto_contract::ContractRef>(contract_id.clone())
            .call(|oracle| oracle.rollup_cond_eq(vec![], vec![], actions.clone()));
        let result = client
            .call(&ink_e2e::bob(), rollup_cond_eq, 0, None)
            .await
            .expect("we should proceed error message");
        // two events : MessageProcessedTo and ErrorReceived
        assert!(result.contains_event("Contracts", "ContractEmitted"));

         */

        Ok(())
    }

    #[ink_e2e::test(
        additional_contracts = "contracts/lotto/Cargo.toml"
    )]
    async fn test_bad_attestor(mut client: ink_e2e::Client<C, E>) -> E2EResult<()> {
        // given
        let contract_id = alice_instantiates_contract(&mut client).await;

        // bob is not granted as attestor => it should not be able to send a message
        let rollup_cond_eq = build_message::<lotto_contract::ContractRef>(contract_id.clone())
            .call(|oracle| oracle.rollup_cond_eq(vec![], vec![], vec![]));
        let result = client.call(&ink_e2e::bob(), rollup_cond_eq, 0, None).await;
        assert!(
            result.is_err(),
            "only attestor should be able to send messages"
        );

        // bob is granted as attestor
        alice_grants_bob_as_attestor(&mut client, &contract_id).await;

        // then bob is able to send a message
        let rollup_cond_eq = build_message::<lotto_contract::ContractRef>(contract_id.clone())
            .call(|oracle| oracle.rollup_cond_eq(vec![], vec![], vec![]));
        let result = client
            .call(&ink_e2e::bob(), rollup_cond_eq, 0, None)
            .await
            .expect("rollup cond eq failed");
        // no event
        assert!(!result.contains_event("Contracts", "ContractEmitted"));

        Ok(())
    }

    #[ink_e2e::test(
        additional_contracts = "contracts/lotto/Cargo.toml"
    )]
    async fn test_bad_messages(mut client: ink_e2e::Client<C, E>) -> E2EResult<()> {
        // given
        let contract_id = alice_instantiates_contract(&mut client).await;

        // bob is granted as attestor
        alice_grants_bob_as_attestor(&mut client, &contract_id).await;

        let actions = vec![HandleActionInput::Reply(58u128.encode())];
        let rollup_cond_eq = build_message::<lotto_contract::ContractRef>(contract_id.clone())
            .call(|oracle| oracle.rollup_cond_eq(vec![], vec![], actions.clone()));
        let result = client.call(&ink_e2e::bob(), rollup_cond_eq, 0, None).await;
        assert!(
            result.is_err(),
            "we should not be able to proceed bad messages"
        );

        Ok(())
    }

    ///
    /// Test the meta transactions
    /// Alice is the owner
    /// Bob is the attestor
    /// Charlie is the sender (ie the payer)
    ///
    #[ink_e2e::test(
        additional_contracts = "contracts/lotto/Cargo.toml"
    )]
    async fn test_meta_tx_rollup_cond_eq(mut client: ink_e2e::Client<C, E>) -> E2EResult<()> {
        let contract_id = alice_instantiates_contract(&mut client).await;

        // Bob is the attestor
        // use the ecsda account because we are not able to verify the sr25519 signature
        let from = ink::primitives::AccountId::from(
            Signer::<PolkadotConfig>::account_id(&subxt_signer::ecdsa::dev::bob()).0,
        );

        // add the role => it should be succeed
        let grant_role = build_message::<lotto_contract::ContractRef>(contract_id.clone())
            .call(|oracle| oracle.grant_role(ATTESTOR_ROLE, Some(from)));
        client
            .call(&ink_e2e::alice(), grant_role, 0, None)
            .await
            .expect("grant the attestor failed");

        // prepare the meta transaction
        let data = RollupCondEqMethodParams::encode(&(vec![], vec![], vec![]));
        let prepare_meta_tx = build_message::<lotto_contract::ContractRef>(contract_id.clone())
            .call(|oracle| oracle.prepare(from, data.clone()));
        let result = client
            .call(&ink_e2e::bob(), prepare_meta_tx, 0, None)
            .await
            .expect("We should be able to prepare the meta tx");

        let (request, _hash) = result
            .return_value()
            .expect("Expected value when preparing meta tx");

        assert_eq!(0, request.nonce);
        assert_eq!(from, request.from);
        assert_eq!(contract_id, request.to);
        assert_eq!(&data, &request.data);

        // Bob signs the message
        let keypair = subxt_signer::ecdsa::dev::bob();
        let signature = keypair.sign(&scale::Encode::encode(&request)).0;

        // do the meta tx: charlie sends the message
        let meta_tx_rollup_cond_eq =
            build_message::<lotto_contract::ContractRef>(contract_id.clone())
                .call(|oracle| oracle.meta_tx_rollup_cond_eq(request.clone(), signature));
        client
            .call(&ink_e2e::charlie(), meta_tx_rollup_cond_eq, 0, None)
            .await
            .expect("meta tx rollup cond eq should not failed");

        // do it again => it must failed
        let meta_tx_rollup_cond_eq =
            build_message::<lotto_contract::ContractRef>(contract_id.clone())
                .call(|oracle| oracle.meta_tx_rollup_cond_eq(request.clone(), signature));
        let result = client
            .call(&ink_e2e::charlie(), meta_tx_rollup_cond_eq, 0, None)
            .await;
        assert!(
            result.is_err(),
            "This message should not be proceed because the nonce is obsolete"
        );

        Ok(())
    }
}
