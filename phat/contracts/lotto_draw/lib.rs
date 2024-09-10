#![cfg_attr(not(feature = "std"), no_std, no_main)]

extern crate alloc;
extern crate core;

#[ink::contract(env = pink_extension::PinkEnvironment)]
mod lotto_draw {
    use alloc::vec::Vec;
    use ink::prelude::{format, string::String};
    use phat_offchain_rollup::clients::ink::{Action, ContractId, InkRollupClient};
    use pink_extension::chain_extension::signing;
    use pink_extension::{debug, error, http_post, info, vrf, ResultExt};
    use scale::{Decode, Encode};
    use serde::Deserialize;
    use serde_json_core;
    use sp_core::crypto::{AccountId32, Ss58Codec};

    pub type RaffleId = u32;
    pub type Number = u16;

    /// Message to request the lotto lotto_draw or the list of winners
    /// message pushed in the queue by the Ink! smart contract and read by the offchain rollup
    #[derive(Eq, PartialEq, Clone, Debug, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct LottoRequestMessage {
        /// lotto_draw number
        raffle_id: RaffleId,
        /// request
        request: Request,
    }

    #[derive(Eq, PartialEq, Clone, Debug, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub enum Request {
        /// request to lotto_draw the n number between min and max values
        /// arg1: number of numbers for the lotto_draw
        /// arg2:  smallest number for the lotto_draw
        /// arg2:  biggest number for the lotto_draw
        DrawNumbers(u8, Number, Number),
        /// request to check if there is a winner for the given numbers
        CheckWinners(Vec<Number>),
    }

    /// Message sent to provide the lotto lotto_draw or the list of winners
    /// response pushed in the queue by the offchain rollup and read by the Ink! smart contract
    #[derive(Encode, Decode, Debug)]
    struct LottoResponseMessage {
        /// initial request
        request: LottoRequestMessage,
        /// response
        response: Response,
    }

    #[derive(Eq, PartialEq, Clone, scale::Encode, scale::Decode, Debug)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub enum Response {
        /// list of numbers
        Numbers(Vec<Number>),
        /// list of winners
        Winners(Vec<AccountId>),
    }

    /// DTO use for serializing and deserializing the json
    #[derive(Deserialize, Encode, Clone, Debug, PartialEq)]
    pub struct IndexerResponse<'a> {
        #[serde(borrow)]
        data: IndexerResponseData<'a>,
    }

    #[derive(Deserialize, Encode, Clone, Debug, PartialEq)]
    #[allow(non_snake_case)]
    struct IndexerResponseData<'a> {
        #[serde(borrow)]
        participations: Participations<'a>,
    }

    #[derive(Deserialize, Encode, Clone, Debug, PartialEq)]
    struct Participations<'a> {
        #[serde(borrow)]
        nodes: Vec<ParticipationNode<'a>>,
    }

    #[derive(Deserialize, Encode, Clone, Debug, PartialEq)]
    #[allow(non_snake_case)]
    struct ParticipationNode<'a> {
        accountId: &'a str,
    }

    #[ink(storage)]
    pub struct Lotto {
        owner: AccountId,
        /// config to send the data to the ink! smart contract
        consumer_config: Option<Config>,
        /// indexer endpoint
        indexer_url: Option<String>,
        /// Key for signing the rollup tx.
        attest_key: [u8; 32],
    }

    #[derive(Encode, Decode, Debug)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    struct Config {
        /// The RPC endpoint of the target blockchain
        rpc: String,
        pallet_id: u8,
        call_id: u8,
        /// The rollup anchor address on the target blockchain
        contract_id: ContractId,
        /// Key for sending out the rollup meta-tx. None to fallback to the wallet based auth.
        sender_key: Option<[u8; 32]>,
    }

    #[derive(Encode, Decode, Debug, PartialEq, Eq)]
    #[repr(u8)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum ContractError {
        BadOrigin,
        ClientNotConfigured,
        InvalidKeyLength,
        InvalidAddressLength,
        NoRequestInQueue,
        FailedToCreateClient,
        FailedToCommitTx,
        FailedToCallRollup,
        // error when checking the winners
        NoNumber,
        IndexerNotConfigured,
        HttpRequestFailed,
        InvalidResponseBody,
        InvalidSs58Address,
        // error when drawing the numbers
        MinGreaterThanMax,
        AddOverFlow,
        SubOverFlow,
        DivByZero,
        // error when verify the numbers
        InvalidContractId,
        CurrentRaffleUnknown,
        UnauthorizedRaffle,
    }

    type Result<T> = core::result::Result<T, ContractError>;

    impl From<phat_offchain_rollup::Error> for ContractError {
        fn from(error: phat_offchain_rollup::Error) -> Self {
            error!("error in the rollup: {:?}", error);
            ContractError::FailedToCallRollup
        }
    }

    impl Lotto {
        #[ink(constructor)]
        pub fn default() -> Self {
            const NONCE: &[u8] = b"lotto";
            let private_key = signing::derive_sr25519_key(NONCE);

            Self {
                owner: Self::env().caller(),
                attest_key: private_key[..32].try_into().expect("Invalid Key Length"),
                consumer_config: None,
                indexer_url: None,
            }
        }

        /// Gets the owner of the contract
        #[ink(message)]
        pub fn owner(&self) -> AccountId {
            self.owner
        }

        /// Gets the attestor address used by this rollup
        #[ink(message)]
        pub fn get_attest_address(&self) -> Vec<u8> {
            signing::get_public_key(&self.attest_key, signing::SigType::Sr25519)
        }

        /// Gets the ecdsa address used by this rollup in the meta transaction
        #[ink(message)]
        pub fn get_attest_ecdsa_address(&self) -> Vec<u8> {
            use ink::env::hash;
            let input = signing::get_public_key(&self.attest_key, signing::SigType::Ecdsa);
            let mut output = <hash::Blake2x256 as hash::HashOutput>::Type::default();
            ink::env::hash_bytes::<hash::Blake2x256>(&input, &mut output);
            output.to_vec()
        }

        /// Gets the sender address used by this rollup (in case of meta-transaction)
        #[ink(message)]
        pub fn get_sender_address(&self) -> Option<Vec<u8>> {
            if let Some(Some(sender_key)) =
                self.consumer_config.as_ref().map(|c| c.sender_key.as_ref())
            {
                let sender_key = signing::get_public_key(sender_key, signing::SigType::Sr25519);
                Some(sender_key)
            } else {
                None
            }
        }

        /// Gets the config of the target consumer contract
        #[ink(message)]
        pub fn get_target_contract(&self) -> Option<(String, u8, u8, ContractId)> {
            self.consumer_config
                .as_ref()
                .map(|c| (c.rpc.clone(), c.pallet_id, c.call_id, c.contract_id))
        }

        /// Configures the target consumer contract (admin only)
        #[ink(message)]
        pub fn config_target_contract(
            &mut self,
            rpc: String,
            pallet_id: u8,
            call_id: u8,
            contract_id: Vec<u8>,
            sender_key: Option<Vec<u8>>,
        ) -> Result<()> {
            self.ensure_owner()?;
            self.consumer_config = Some(Config {
                rpc,
                pallet_id,
                call_id,
                contract_id: contract_id
                    .try_into()
                    .or(Err(ContractError::InvalidAddressLength))?,
                sender_key: match sender_key {
                    Some(key) => Some(key.try_into().or(Err(ContractError::InvalidKeyLength))?),
                    None => None,
                },
            });
            Ok(())
        }

        /// Gets the config to target the indexer
        #[ink(message)]
        pub fn get_indexer_url(&self) -> Option<String> {
            self.indexer_url.clone()
        }

        /// Configures the indexer (admin only)
        #[ink(message)]
        pub fn config_indexer(&mut self, indexer_url: String) -> Result<()> {
            self.ensure_owner()?;
            self.indexer_url = Some(indexer_url);
            Ok(())
        }

        /// Transfers the ownership of the contract (admin only)
        #[ink(message)]
        pub fn transfer_ownership(&mut self, new_owner: AccountId) -> Result<()> {
            self.ensure_owner()?;
            self.owner = new_owner;
            Ok(())
        }

        /// Processes a request by a rollup transaction
        #[ink(message)]
        pub fn answer_request(&self) -> Result<Option<Vec<u8>>> {
            let config = self.ensure_client_configured()?;
            let mut client = connect(config)?;

            // Get a request if presents
            let request: LottoRequestMessage = client
                .pop()
                .log_err("answer_request: failed to read queue")?
                .ok_or(ContractError::NoRequestInQueue)?;

            ink::env::debug_println!("Received request: {request:02x?}");

            let response = self.handle_request(request)?;
            // Attach an action to the tx by:
            client.action(Action::Reply(response.encode()));

            maybe_submit_tx(client, &self.attest_key, config.sender_key.as_ref())
        }

        fn handle_request(&self, message: LottoRequestMessage) -> Result<LottoResponseMessage> {
            let response = match message.request {
                Request::DrawNumbers(nb_numbers, smallest_number, biggest_number) => self
                    .inner_get_numbers(
                        message.raffle_id,
                        nb_numbers,
                        smallest_number,
                        biggest_number,
                    )
                    .map(Response::Numbers)?,
                Request::CheckWinners(ref numbers) => self
                    .inner_get_winners(message.raffle_id, numbers)
                    .map(Response::Winners)?,
            };

            Ok(LottoResponseMessage {
                request: message,
                response,
            })
        }

        /// Verify if the winning numbers for a raffle are valid (only for past raffles)
        #[ink(message)]
        pub fn verify_numbers(
            &self,
            contract_id: ContractId,
            raffle_id: RaffleId,
            nb_numbers: u8,
            smallest_number: Number,
            biggest_number: Number,
            numbers: Vec<Number>,
        ) -> Result<bool> {
            let config = self.ensure_client_configured()?;

            // check if the target contract is correct
            if contract_id != config.contract_id {
                return Err(ContractError::InvalidContractId);
            }

            let mut client = connect(config)?;

            const LAST_RAFFLE_FOR_VERIF: u32 = ink::selector_id!("LAST_RAFFLE_FOR_VERIF");

            let last_raffle: RaffleId = client
                .get(&LAST_RAFFLE_FOR_VERIF)
                .log_err("verify numbers: last raffle unknown")?
                .ok_or(ContractError::CurrentRaffleUnknown)?;

            // verify the winning numbers only for the past raffles
            if raffle_id > last_raffle {
                return Err(ContractError::UnauthorizedRaffle);
            }

            self.inner_verify_numbers(
                raffle_id,
                nb_numbers,
                smallest_number,
                biggest_number,
                numbers,
            )
        }

        pub fn inner_verify_numbers(
            &self,
            raffle_id: RaffleId,
            nb_numbers: u8,
            smallest_number: Number,
            biggest_number: Number,
            numbers: Vec<Number>,
        ) -> Result<bool> {
            let winning_numbers =
                self.inner_get_numbers(raffle_id, nb_numbers, smallest_number, biggest_number)?;
            if winning_numbers.len() != numbers.len() {
                return Ok(false);
            }

            for n in &numbers {
                if !winning_numbers.contains(n) {
                    return Ok(false);
                }
            }

            Ok(true)
        }

        fn inner_get_numbers(
            &self,
            raffle_id: RaffleId,
            nb_numbers: u8,
            smallest_number: Number,
            biggest_number: Number,
        ) -> Result<Vec<Number>> {
            info!(
                "Request received for raffle {raffle_id} - draw {nb_numbers} numbers between {smallest_number} and {biggest_number}"
            );

            let contract_id = self.ensure_client_configured()?.contract_id;

            if smallest_number > biggest_number {
                return Err(ContractError::MinGreaterThanMax);
            }

            let mut numbers = Vec::new();
            let mut i: u8 = 0;

            while numbers.len() < nb_numbers as usize {
                // build a salt for this lotto_draw number
                let mut salt: Vec<u8> = Vec::new();
                salt.extend_from_slice(&i.to_be_bytes());
                salt.extend_from_slice(&raffle_id.to_be_bytes());
                salt.extend_from_slice(&contract_id);

                // lotto_draw the number
                let number = self.inner_get_number(salt, smallest_number, biggest_number)?;
                // check if the number has already been drawn
                if !numbers.iter().any(|&n| n == number) {
                    // the number has not been drawn yet => we added it
                    numbers.push(number);
                }
                //i += 1;
                i = i.checked_add(1).ok_or(ContractError::AddOverFlow)?;
            }

            info!("Numbers: {numbers:?}");

            Ok(numbers)
        }

        fn inner_get_number(&self, salt: Vec<u8>, min: Number, max: Number) -> Result<Number> {
            let output = vrf(&salt);
            // keep only 8 bytes to compute the random u64
            let mut arr = [0x00; 8];
            arr.copy_from_slice(&output[0..8]);
            let rand_u64 = u64::from_le_bytes(arr);

            // r = rand_u64() % (max - min + 1) + min
            // use u128 because (max - min + 1) can be equal to (U64::MAX - 0 + 1)
            let a = (max as u128)
                .checked_sub(min as u128)
                .ok_or(ContractError::SubOverFlow)?
                .checked_add(1u128)
                .ok_or(ContractError::AddOverFlow)?;
            //let b = (rand_u64 as u128) % a;
            let b = (rand_u64 as u128).checked_rem_euclid(a).ok_or(ContractError::DivByZero)?;
            let r = b
                .checked_add(min as u128)
                .ok_or(ContractError::AddOverFlow)?;

            Ok(r as Number)
        }

        fn inner_get_winners(
            &self,
            raffle_id: RaffleId,
            numbers: &Vec<Number>,
        ) -> Result<Vec<AccountId>> {
            info!(
                "Request received to get the winners for raffle id {raffle_id} and numbers {numbers:?} "
            );

            if numbers.is_empty() {
                return Err(ContractError::NoNumber);
            }

            // check if the endpoint is configured
            let indexer_endpoint = self.ensure_indexer_configured()?;

            // build the headers
            let headers = alloc::vec![
                ("Content-Type".into(), "application/json".into()),
                ("Accept".into(), "application/json".into())
            ];
            // build the filter
            let mut filter = format!(
                r#"filter:{{and:[{{numRaffle:{{equalTo:\"{}\"}}}}"#,
                raffle_id
            );
            for n in numbers {
                let f = format!(r#",{{numbers:{{contains:\"{}\"}}}}"#, n);
                filter.push_str(&f);
            }
            filter.push_str("]}");

            // build the body
            let body = format!(
                r#"{{"query" : "{{participations({}){{ nodes {{ accountId }} }} }}"}}"#,
                filter
            );

            debug!("body: {body}");

            // query the indexer
            let resp = http_post!(indexer_endpoint, body, headers);

            // check the result
            if resp.status_code != 200 {
                ink::env::debug_println!("status code {}", resp.status_code);
                return Err(ContractError::HttpRequestFailed);
            }

            // parse the result
            let result: IndexerResponse = serde_json_core::from_slice(resp.body.as_slice())
                .or(Err(ContractError::InvalidResponseBody))?
                .0;

            // add the winners
            let mut winners = Vec::new();
            for w in result.data.participations.nodes.iter() {
                // build the accountId from the string address
                let account_id = AccountId32::from_ss58check(w.accountId)
                    .or(Err(ContractError::InvalidSs58Address))?;
                let address_hex: [u8; 32] = scale::Encode::encode(&account_id)
                    .try_into()
                    .or(Err(ContractError::InvalidKeyLength))?;
                winners.push(AccountId::from(address_hex));
            }

            info!("Winners: {winners:02x?}");

            Ok(winners)
        }

        /// Returns BadOrigin error if the caller is not the owner
        fn ensure_owner(&self) -> Result<()> {
            if self.env().caller() == self.owner {
                Ok(())
            } else {
                Err(ContractError::BadOrigin)
            }
        }

        /// Returns the config reference or raise the error `ClientNotConfigured`
        fn ensure_client_configured(&self) -> Result<&Config> {
            self.consumer_config
                .as_ref()
                .ok_or(ContractError::ClientNotConfigured)
        }

        fn ensure_indexer_configured(&self) -> Result<&String> {
            self.indexer_url
                .as_ref()
                .ok_or(ContractError::IndexerNotConfigured)
        }
    }

    fn connect(config: &Config) -> Result<InkRollupClient> {
        let result = InkRollupClient::new(
            &config.rpc,
            config.pallet_id,
            config.call_id,
            &config.contract_id,
        )
        .log_err("failed to create rollup client");

        match result {
            Ok(client) => Ok(client),
            Err(e) => {
                error!("Error : {:?}", e);
                Err(ContractError::FailedToCreateClient)
            }
        }
    }

    fn maybe_submit_tx(
        client: InkRollupClient,
        attest_key: &[u8; 32],
        sender_key: Option<&[u8; 32]>,
    ) -> Result<Option<Vec<u8>>> {
        let maybe_submittable = client
            .commit()
            .log_err("failed to commit")
            .map_err(|_| ContractError::FailedToCommitTx)?;

        if let Some(submittable) = maybe_submittable {
            let tx_id = if let Some(sender_key) = sender_key {
                // Prefer to meta-tx
                submittable
                    .submit_meta_tx(attest_key, sender_key)
                    .log_err("failed to submit rollup meta-tx")?
            } else {
                // Fallback to account-based authentication
                submittable
                    .submit(attest_key)
                    .log_err("failed to submit rollup tx")?
            };
            return Ok(Some(tx_id));
        }
        Ok(None)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        struct EnvVars {
            /// The RPC endpoint of the target blockchain
            rpc: String,
            pallet_id: u8,
            call_id: u8,
            /// The rollup anchor address on the target blockchain
            contract_id: ContractId,
            /// When we want to manually set the attestor key for signing the message (only dev purpose)
            //attest_key: Vec<u8>,
            /// When we want to use meta tx
            sender_key: Option<Vec<u8>>,
        }

        fn get_env(key: &str) -> String {
            std::env::var(key).expect("env not found")
        }

        fn config() -> EnvVars {
            dotenvy::dotenv().ok();
            let rpc = get_env("RPC");
            let pallet_id: u8 = get_env("PALLET_ID").parse().expect("u8 expected");
            let call_id: u8 = get_env("CALL_ID").parse().expect("u8 expected");
            let contract_id: ContractId = hex::decode(get_env("CONTRACT_ID"))
                .expect("hex decode failed")
                .try_into()
                .expect("incorrect length");
            //let attest_key = hex::decode(get_env("ATTEST_KEY")).expect("hex decode failed");
            let sender_key = std::env::var("SENDER_KEY")
                .map(|s| hex::decode(s).expect("hex decode failed"))
                .ok();

            EnvVars {
                rpc: rpc.to_string(),
                pallet_id,
                call_id,
                contract_id: contract_id.into(),
                //attest_key,
                sender_key,
            }
        }

        fn init_contract() -> Lotto {
            let EnvVars {
                rpc,
                pallet_id,
                call_id,
                contract_id,
                //attest_key,
                sender_key,
            } = config();

            let mut lotto = Lotto::default();
            lotto
                .config_target_contract(rpc, pallet_id, call_id, contract_id.into(), sender_key)
                .unwrap();

            lotto
                .config_indexer("https://query.substrate.fi/lotto-subquery-shibuya".to_string())
                .unwrap();
            //lotto.set_attest_key(Some(attest_key)).unwrap();

            lotto
        }

        #[ink::test]
        fn test_get_numbers() {
            let _ = env_logger::try_init();
            pink_extension_runtime::mock_ext::mock_all_ext();

            let lotto = init_contract();

            let raffle_id = 1;
            let nb_numbers = 5;
            let smallest_number = 1;
            let biggest_number = 50;

            let result = lotto
                .inner_get_numbers(raffle_id, nb_numbers, smallest_number, biggest_number)
                .unwrap();
            assert_eq!(nb_numbers as usize, result.len());
            for &n in result.iter() {
                assert!(n >= smallest_number);
                assert!(n <= biggest_number);
            }

            ink::env::debug_println!("random numbers: {result:?}");
        }

        #[ink::test]
        fn test_get_numbers_from_1_to_5() {
            let _ = env_logger::try_init();
            pink_extension_runtime::mock_ext::mock_all_ext();

            let lotto = init_contract();

            let raffle_id = 1;
            let nb_numbers = 5;
            let smallest_number = 1;
            let biggest_number = 5;

            let result = lotto
                .inner_get_numbers(raffle_id, nb_numbers, smallest_number, biggest_number)
                .unwrap();
            assert_eq!(nb_numbers as usize, result.len());
            for &n in result.iter() {
                assert!(n >= smallest_number);
                assert!(n <= biggest_number);
            }

            ink::env::debug_println!("random numbers: {result:?}");
        }

        #[ink::test]
        fn test_with_different_draw_num() {
            let _ = env_logger::try_init();
            pink_extension_runtime::mock_ext::mock_all_ext();

            let lotto = init_contract();

            let nb_numbers = 5;
            let smallest_number = 1;
            let biggest_number = 50;

            let mut results = Vec::new();

            for i in 0..100 {
                let result = lotto
                    .inner_get_numbers(i, nb_numbers, smallest_number, biggest_number)
                    .unwrap();
                // this result must be different from the previous ones
                results.iter().for_each(|r| assert_ne!(result, *r));

                // same request message means same result
                let result_2 = lotto
                    .inner_get_numbers(i, nb_numbers, smallest_number, biggest_number)
                    .unwrap();
                assert_eq!(result, result_2);

                results.push(result);
            }
        }

        #[ink::test]
        fn test_verify_numbers() {
            let _ = env_logger::try_init();
            pink_extension_runtime::mock_ext::mock_all_ext();

            let lotto = init_contract();

            let raffle_id = 1;
            let nb_numbers = 5;
            let smallest_number = 1;
            let biggest_number = 50;

            let numbers = lotto
                .inner_get_numbers(raffle_id, nb_numbers, smallest_number, biggest_number)
                .unwrap();

            assert_eq!(
                Ok(true),
                lotto.inner_verify_numbers(
                    raffle_id,
                    nb_numbers,
                    smallest_number,
                    biggest_number,
                    numbers.clone()
                )
            );

            assert_eq!(
                Ok(false),
                lotto.inner_verify_numbers(
                    raffle_id + 1,
                    nb_numbers,
                    smallest_number,
                    biggest_number,
                    numbers.clone()
                )
            );
        }

        #[ink::test]
        fn test_verify_numbers_with_bad_contract_id() {
            let _ = env_logger::try_init();
            pink_extension_runtime::mock_ext::mock_all_ext();

            let mut lotto = init_contract();

            let raffle_id = 1;
            let nb_numbers = 5;
            let smallest_number = 1;
            let biggest_number = 50;

            let numbers = lotto
                .inner_get_numbers(raffle_id, nb_numbers, smallest_number, biggest_number)
                .unwrap();

            assert_eq!(
                Ok(true),
                lotto.inner_verify_numbers(
                    raffle_id,
                    nb_numbers,
                    smallest_number,
                    biggest_number,
                    numbers.clone()
                )
            );

            let target_contract = lotto.get_target_contract().unwrap();

            let bad_contract_id: ContractId = [0; 32];
            lotto
                .config_target_contract(
                    target_contract.0,
                    target_contract.1,
                    target_contract.2,
                    bad_contract_id.to_vec(),
                    None,
                )
                .unwrap();

            assert_eq!(
                Ok(false),
                lotto.inner_verify_numbers(
                    raffle_id,
                    nb_numbers,
                    smallest_number,
                    biggest_number,
                    numbers.clone()
                )
            );
        }

        #[ink::test]
        fn test_get_winners() {
            let _ = env_logger::try_init();
            pink_extension_runtime::mock_ext::mock_all_ext();

            let lotto = init_contract();

            let draw_num = 2;
            let numbers = vec![15, 1, 44, 28];

            let winners = lotto.inner_get_winners(draw_num, &numbers).unwrap();
            ink::env::debug_println!("winners: {winners:?}");
        }

        #[ink::test]
        fn test_no_winner() {
            let _ = env_logger::try_init();
            pink_extension_runtime::mock_ext::mock_all_ext();

            let lotto = init_contract();

            let draw_num = 0;
            let numbers = vec![150, 1, 44, 2800];

            let winners = lotto.inner_get_winners(draw_num, &numbers).unwrap();
            assert_eq!(0, winners.len());
        }

        #[ink::test]
        fn test_no_number() {
            let _ = env_logger::try_init();
            pink_extension_runtime::mock_ext::mock_all_ext();

            let lotto = init_contract();

            let draw_num = 0;
            let numbers = vec![];

            let result = lotto.inner_get_winners(draw_num, &numbers);
            assert_eq!(Err(ContractError::NoNumber), result);
        }

        #[ink::test]
        #[ignore = "The target contract must be deployed on the Substrate node and a random number request must be submitted"]
        fn answer_request() {
            let _ = env_logger::try_init();
            pink_extension_runtime::mock_ext::mock_all_ext();

            let lotto = init_contract();

            let r = lotto.answer_request().expect("failed to answer request");
            ink::env::debug_println!("answer request: {r:?}");
        }


        #[ink::test]
        fn encode_response_numbers() {
            let _ = env_logger::try_init();
            pink_extension_runtime::mock_ext::mock_all_ext();

            let lotto = init_contract();

            //Request received for raffle 6 - draw 4 numbers between 1 and 50
            // Numbers: [4, 49, 41, 16]

            let raffle_id = 6;
            let numbers = vec![4, 49, 41, 16];

            let response = LottoResponseMessage {
                request: LottoRequestMessage {raffle_id, request: Request::DrawNumbers(4, 1, 50)},
                response: Response::Numbers(numbers.clone()),
            };
            let encoded_response = response.encode();
            ink::env::debug_println!("Reply response numbers: {encoded_response:02x?}");

            let response = LottoResponseMessage {
                request: LottoRequestMessage {raffle_id, request: Request::CheckWinners(numbers)},
                response: Response::Winners(vec![]),
            };
            let encoded_response = response.encode();
            ink::env::debug_println!("Reply response winners: {encoded_response:02x?}");

        }


        #[ink::test]
        fn encode_keys() {

            const QUEUE_PREFIX : &[u8] = b"q/";

            const QUEUE_HEAD_KEY : &[u8] = b"_head";
            let head_key = [QUEUE_PREFIX, QUEUE_HEAD_KEY].concat();
            ink::env::debug_println!("queue head key: {head_key:02x?}");

            const QUEUE_TAIL_KEY : &[u8] = b"_tail";
            let tail_key = [QUEUE_PREFIX, QUEUE_TAIL_KEY].concat();
            ink::env::debug_println!("queue tail key: {tail_key:02x?}");

            let id: u32 = 11;
            let key = [QUEUE_PREFIX, &id.encode()].concat();
            ink::env::debug_println!("queue key: {key:02x?}");

        }

        #[ink::test]
        fn decode_message() {
            let encoded_message : Vec<u8> = hex::decode("0600000001100400310029001000").expect("hex decode failed");
            let message = LottoRequestMessage::decode(&mut encoded_message.as_slice());
            ink::env::debug_println!("message: {message:?}");

            let encoded_message : Vec<u8> = hex::decode("07000000000401003200").expect("hex decode failed");
            let message = LottoRequestMessage::decode(&mut encoded_message.as_slice());
            ink::env::debug_println!("message: {message:?}");

        }


    }
}
