use std::{collections::BTreeMap, path::PathBuf};

use casper_engine_test_support::{
    DEFAULT_RUN_GENESIS_REQUEST,
    InMemoryWasmTestBuilder,
};
use casper_types::{account::AccountHash, ContractHash, ContractPackageHash, Key, runtime_args, RuntimeArgs, U512, U256, CLTyped};
use casper_types::bytesrepr::FromBytes;
use cep47::TokenId;
use maplit::btreemap;
use casper_private_auction_core::accounts::GIFT_ACCOUNT;

use crate::{
    utils::{deploy, DeploySource, fund_account, query, query_dictionary_item, create_account},
};
use crate::utils::key_and_value_to_str;

pub struct GiftContract {
    pub builder: InMemoryWasmTestBuilder,
    pub gift_contract: (ContractHash, ContractPackageHash),
    pub nft: (ContractHash, ContractPackageHash),
    pub accounts: (AccountHash, AccountHash, AccountHash, AccountHash, AccountHash),
}

impl GiftContract {

    pub fn deploy() -> Self {
        let admin = AccountHash::from_formatted_str(GIFT_ACCOUNT).unwrap();
        let tim = create_account();
        let ali = create_account();
        let bob = create_account();
        let dan = create_account();

        let mut builder = InMemoryWasmTestBuilder::default();
        let base_amount = U512::from(50_000_000_000_000_u64);
        let empty_amount = U512::from(1_u64);

        builder.run_genesis(&DEFAULT_RUN_GENESIS_REQUEST).commit();
        builder.exec(fund_account(&admin, base_amount.clone())).expect_success().commit();
        builder.exec(fund_account(&tim, empty_amount.clone())).expect_success().commit();
        builder.exec(fund_account(&ali, base_amount.clone())).expect_success().commit();
        builder.exec(fund_account(&bob, empty_amount.clone())).expect_success().commit();
        builder.exec(fund_account(&dan, empty_amount.clone())).expect_success().commit();

        let (_, kyc_package) = Self::deploy_kyc(&mut builder, &admin);
        Self::add_kyc(&mut builder, &kyc_package, &admin, &admin);
        Self::add_kyc(&mut builder, &kyc_package, &admin, &tim);
        Self::add_kyc(&mut builder, &kyc_package, &admin, &ali);
        Self::add_kyc(&mut builder, &kyc_package, &admin, &bob);
        // No kyc for dan

        let (nft_hash, nft_package) = Self::deploy_nft(&mut builder, &admin, kyc_package);

        let token_meta = btreemap! {
            "origin".to_string() => "fire".to_string()
        };

        // Get the configured commissions
        let commissions = BTreeMap::new();

        // Mint several tokens
        Self::mint_nft(
            &mut builder,
            &nft_package,
            &Key::Account(tim),
            token_meta.clone(),
            &admin,
            commissions.clone(),
        );

        Self::mint_nft(
            &mut builder,
            &nft_package,
            &Key::Account(ali),
            token_meta.clone(),
            &admin,
            commissions.clone(),
        );

        Self::mint_nft(
            &mut builder,
            &nft_package,
            &Key::Account(bob),
            token_meta,
            &admin,
            commissions,
        );
        //
        // let token_id_1 = Self::get_token_by_index(&builder, &nft_hash, &tim, U256::zero()).unwrap();
        // let token_id_2 = Self::get_token_by_index(&builder, &nft_hash, &ali, U256::one()).unwrap();
        // let token_id_3 = Self::get_token_by_index(&builder, &nft_hash, &bob, U256::zero()).unwrap();

        // auction_args.set_beneficiary(&admin);
        // auction_args.set_token_contract_hash(&nft_package);
        // auction_args.set_kyc_package_hash(&kyc_package);
        // auction_args.set_synth_package_hash(&synth_package);
        // auction_args.set_token_id(&token_id);

        let (gift_hash, gift_package) =
            Self::deploy_gift("gift-installer.wasm".to_string(), &mut builder, &admin);

        Self {
            builder,
            gift_contract: (gift_hash, gift_package),
            nft: (nft_hash, nft_package),
            accounts: (admin, tim, ali, bob, dan),
        }
    }

    pub fn transfer_funds(&mut self, account: &AccountHash, amount: U512) {
        self.builder.exec(fund_account(&account, amount)).expect_success().commit();
    }

    pub fn deploy_kyc(
        builder: &mut InMemoryWasmTestBuilder,
        admin: &AccountHash,
    ) -> (ContractHash, ContractPackageHash) {
        let mut meta = BTreeMap::new();
        meta.insert("origin".to_string(), "kyc".to_string());

        let kyc_args = runtime_args! {
            "name" => "kyc",
            "contract_name" => "kyc",
            "symbol" => "symbol",
            "meta" => meta,
            "admin" => Key::Account(*admin)
        };
        let auction_code = PathBuf::from("kyc-contract.wasm");
        deploy(
            builder,
            admin,
            &DeploySource::Code(auction_code),
            kyc_args,
            true,
            None,
        );

        let contract_hash = query(
            builder,
            Key::Account(*admin),
            &["kyc_contract_hash_wrapped".to_string()],
        );
        let contract_package = query(
            builder,
            Key::Account(*admin),
            &["kyc_package_hash_wrapped".to_string()],
        );

        (contract_hash, contract_package)
    }

    pub fn deploy_nft(
        builder: &mut InMemoryWasmTestBuilder,
        admin: &AccountHash,
        kyc_package_hash: ContractPackageHash,
    ) -> (ContractHash, ContractPackageHash) {
        let token_args = runtime_args! {
            "name" => "token",
            "symbol" => "TK",
            "meta" => btreemap! {
                "origin".to_string() => "fire".to_string()
            },
            "admin" => Key::Account(*admin),
            "kyc_package_hash" => Some(Key::Hash(kyc_package_hash.value())),
            "contract_name" => "NFT".to_string()
        };
        let nft_code = PathBuf::from("metacask-nft.wasm");
        deploy(
            builder,
            admin,
            &DeploySource::Code(nft_code),
            token_args,
            true,
            None,
        );

        let contract_hash: ContractHash = query(
            builder,
            Key::Account(*admin),
            &["NFT_contract_hash_wrapped".to_string()],
        );
        let contract_package: ContractPackageHash = query(
            builder,
            Key::Account(*admin),
            &["NFT_package_hash_wrapped".to_string()],
        );
        (contract_hash, contract_package)
    }

    pub fn deploy_gift(
        wasm: String,
        builder: &mut InMemoryWasmTestBuilder,
        admin: &AccountHash,
    ) -> (ContractHash, ContractPackageHash) {
        let gift_code = PathBuf::from(wasm);
        let deploy_code = DeploySource::Code(gift_code);
        deploy(
            builder,
            admin,
            &deploy_code,
            runtime_args! {
                "contract_name" => "test"
            },
            true,
            None,
        );

        let contract_hash: ContractHash = query(
            builder,
            Key::Account(*admin),
            &["test_contract_hash_wrapped".to_string()],
        );
        let contract_package: ContractPackageHash = query(
            builder,
            Key::Account(*admin),
            &["test_package_hash_wrapped".to_string()],
        );
        (contract_hash, contract_package)
    }
    //
    // pub fn mint_nft_token(
    //     &mut self,
    //     recipient: &Key,
    //     token_id: &str,
    //     token_meta: &BTreeMap<String, String>,
    //     sender: &AccountHash,
    //     commissions: BTreeMap<String, String>,
    // ) {
    //     Self::mint_nft(
    //         &mut self.builder,
    //         &self.nft_package,
    //         recipient,
    //         token_id,
    //         token_meta,
    //         sender,
    //         commissions,
    //     )
    // }

    pub fn mint_nft(
        builder: &mut InMemoryWasmTestBuilder,
        nft_package: &ContractPackageHash,
        recipient: &Key,
        token_meta: BTreeMap<String, String>,
        sender: &AccountHash,
        commissions: BTreeMap<String, String>,
    ) {
        let args = runtime_args! {
            "recipient" => *recipient,
            "token_meta" => token_meta,
            "token_commission" => commissions,
        };
        deploy(
            builder,
            sender,
            &DeploySource::ByPackageHash {
                package_hash: *nft_package,
                method: "mint".to_string(),
            },
            args,
            true,
            None,
        );
    }

    // pub fn add_kyc_token(&mut self, recipient: &AccountHash) {
    //     Self::add_kyc(&mut self.builder, &self.kyc_package, &self.admin, recipient)
    // }

    pub fn add_kyc(
        builder: &mut InMemoryWasmTestBuilder,
        kyc_package: &ContractPackageHash,
        admin: &AccountHash,
        recipient: &AccountHash,
    ) {
        let mut token_meta = BTreeMap::new();
        token_meta.insert("status".to_string(), "active".to_string());
        let args = runtime_args! {
            "recipient" => Key::Account(*recipient),
            "token_id" => Some(recipient.to_string()),
            "token_meta" => token_meta,
        };

        deploy(
            builder,
            admin,
            &DeploySource::ByPackageHash {
                package_hash: *kyc_package,
                method: "mint".to_string(),
            },
            args,
            true,
            None,
        );
    }

    pub fn gift(&mut self, caller: &AccountHash, sender: &AccountHash, token_id: String, time: u64) {
        // self.call(caller, "gift", runtime_args! {
        //     "sender" => Key::Account(sender.clone()),
        //     "token_id" => token_id,
        //     "token_package_hash" => Key::Hash(token_package_hash.value())
        // }, time)
        let session_code = PathBuf::from("gift-installer-test.wasm");
        deploy(
            &mut self.builder,
            caller,
            &DeploySource::Code(session_code),
            runtime_args! {
                "sender" => Key::Account(sender.clone()),
                "token_id" => token_id,
                "token_package_hash" => Key::Hash(self.nft.1.value()),
                "gift_contract" => Key::Hash(self.gift_contract.1.value())
            },
            true,
            Some(time),
        );
    }

    pub fn cancel(&mut self, caller: &AccountHash, token_id: String, time: u64) {
        self.call(caller, "cancel", runtime_args! {
            "token_id" => token_id,
        }, time)
    }

    pub fn claim(&mut self, caller: &AccountHash, receiver: &AccountHash, token_id: String, time: u64) {
        self.call(caller, "claim", runtime_args! {
            "receiver" => Key::Account(receiver.clone()),
            "token_id" => token_id,
        }, time)
    }

    pub fn owner_of(&self, token_id: TokenId) -> Option<Key> {
        self.query_dictionary("owners", token_id)
    }

    fn query_dictionary<T: CLTyped + FromBytes>(
        &self,
        dict_name: &str,
        key: String,
    ) -> Option<T> {
        // self.env
        //     .query_dictionary(self.nft.0.clone(), dict_name.to_string(), key)
        query_dictionary_item(&self.builder,
                              Key::Hash(self.nft.0.value()),
                              Some(dict_name.to_string()),
                              key
        )
            .expect("should be stored value.")
            .as_cl_value()
            .expect("should be cl value.")
            .clone()
            .into_t()
            .expect("Wrong type in query result.")
    }

    /// Wrapper function for calling an entrypoint on the contract with the access rights of the deployer.
    pub fn call(&mut self, caller: &AccountHash, method: &str, args: RuntimeArgs, time: u64) {
        deploy(
            &mut self.builder,
            caller,
            &DeploySource::ByPackageHash {
                package_hash: self.gift_contract.1.clone(),
                method: method.to_string(),
            },
            args,
            true,
            Some(time),
        );
    }

    pub fn get_token_by_index(&self, account: &AccountHash, index: U256) -> Option<TokenId> {
        query_dictionary_item(&self.builder,
                              Key::Hash(self.nft.0.value()),
                              Some("owned_tokens_by_index".to_string()),
                              key_and_value_to_str(&Key::Account(account.clone()), &index)
        )
            .expect("should be stored value.")
            .as_cl_value()
            .expect("should be cl value.")
            .clone()
            .into_t()
            .expect("Wrong type in query result.")
    }
    //
    // fn query_dictionary_value<T: CLTyped + FromBytes>(
    //     &self,
    //     base: Key,
    //     dict_name: &str,
    //     key: String,
    // ) -> Option<T> {
    //     query_dictionary_item(&self.builder, base, Some(dict_name.to_string()), key)
    //         .expect("should be stored value.")
    //         .as_cl_value()
    //         .expect("should be cl value.")
    //         .clone()
    //         .into_t()
    //         .expect("Wrong type in query result.")
    // }
}
