use std::{collections::BTreeMap, path::PathBuf};

use casper_engine_test_support::{
    DEFAULT_RUN_GENESIS_REQUEST,
    InMemoryWasmTestBuilder,
};
use casper_types::{
    account::AccountHash, bytesrepr::FromBytes, CLTyped, ContractHash, ContractPackageHash,
    Key, runtime_args, RuntimeArgs, U512,
};
use maplit::btreemap;
use casper_private_auction_core::accounts::MARKETPLACE_ACCOUNT;

use casper_private_auction_core::keys;

use crate::{
    utils::{deploy, DeploySource, fund_account, query, query_dictionary_item, create_account},
};

pub trait BaseAuctionArgs {
    fn build(&self) -> RuntimeArgs;
    fn set_start_time(&mut self, time: u64);
    fn set_cancel_time(&mut self, _time: Option<u64>) {
        // do-nothing
    }
    fn set_end_time(&mut self, time: u64);
    fn set_start_price(&mut self, _price: U512) {
        // do nothing
    }
    fn set_swap_price(&mut self, _price: U512) {
        // do nothing
    }
    fn set_reserve_price(&mut self, _price: U512) {
        // do-nothing
    }
    fn set_beneficiary(&mut self, account: &AccountHash);
    fn set_token_contract_hash(&mut self, hash: &ContractPackageHash);
    fn set_kyc_package_hash(&mut self, hash: &ContractPackageHash);
    fn set_synth_package_hash(&mut self, hash: &ContractPackageHash);
    fn set_token_id(&mut self, token_id: &String);
    fn get_nft_commission(&self) -> u32;
    fn get_wasm(&self) -> String;
}

pub struct AuctionContract {
    pub builder: InMemoryWasmTestBuilder,
    pub auction_contract: (ContractHash, ContractPackageHash),
    pub nft: (ContractHash, ContractPackageHash),
    pub kyc: (ContractHash, ContractPackageHash),
    pub synth: (ContractHash, ContractPackageHash),
    pub accounts: (AccountHash, AccountHash, AccountHash, AccountHash, AccountHash, AccountHash),
}

impl AuctionContract {

    pub fn deploy(auction_args: &mut impl BaseAuctionArgs) -> Self {
        let admin = create_account();
        let market = AccountHash::from_formatted_str(MARKETPLACE_ACCOUNT).unwrap();
        let artist = create_account();
        let ali = create_account();
        let bob = create_account();
        let dan = create_account();

        let mut builder = InMemoryWasmTestBuilder::default();
        let base_amount = U512::from(50_000_000_000_000_u64);
        let empty_amount = U512::from(1_u64);

        builder.run_genesis(&DEFAULT_RUN_GENESIS_REQUEST).commit();
        builder.exec(fund_account(&admin, base_amount.clone())).expect_success().commit();
        builder.exec(fund_account(&market, empty_amount)).expect_success().commit();
        builder.exec(fund_account(&artist, empty_amount)).expect_success().commit();
        builder.exec(fund_account(&ali, base_amount.clone())).expect_success().commit();
        builder.exec(fund_account(&bob, base_amount.clone())).expect_success().commit();
        builder.exec(fund_account(&dan, base_amount.clone())).expect_success().commit();

        let (kyc_hash, kyc_package) = Self::deploy_kyc(&mut builder, &admin);
        Self::add_kyc(&mut builder, &kyc_package, &admin, &admin);
        Self::add_kyc(&mut builder, &kyc_package, &admin, &artist);
        Self::add_kyc(&mut builder, &kyc_package, &admin, &ali);
        Self::add_kyc(&mut builder, &kyc_package, &admin, &bob);
        // No kyc for dan

        let (synth_hash, synth_package) = Self::deploy_synth(&mut builder, &admin);
        Self::add_synth(&mut builder, &synth_package, &admin, &admin);
        Self::add_synth(&mut builder, &synth_package, &admin, &ali);
        // no synth for bob
        // No synth for ali

        let (nft_hash, nft_package) = Self::deploy_nft(&mut builder, &admin, kyc_package);

        let token_id = String::from("custom_token_id");
        let token_meta = btreemap! {
            "origin".to_string() => "fire".to_string()
        };

        // Get the configured commissions
        let com = auction_args.get_nft_commission();

        let mut commissions = BTreeMap::new();
        commissions.insert("artist_account".to_string(), artist.to_formatted_string());
        commissions.insert("artist_rate".to_string(), com.to_string());

        Self::mint_nft(
            &mut builder,
            &nft_package,
            &Key::Account(admin),
            &token_id,
            &token_meta,
            &admin,
            commissions,
        );

        auction_args.set_beneficiary(&admin);
        auction_args.set_token_contract_hash(&nft_package);
        auction_args.set_kyc_package_hash(&kyc_package);
        auction_args.set_synth_package_hash(&synth_package);
        auction_args.set_token_id(&token_id);

        let (auction_hash, auction_package) =
            Self::deploy_auction(auction_args.get_wasm(), &mut builder, &admin, auction_args.build());

        Self {
            builder,
            auction_contract: (auction_hash, auction_package),
            nft: (nft_hash, nft_package),
            kyc: (kyc_hash, kyc_package),
            synth: (synth_hash, synth_package),
            accounts: (admin, market, artist, ali, bob, dan),
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

    pub fn deploy_synth(
        builder: &mut InMemoryWasmTestBuilder,
        admin: &AccountHash,
    ) -> (ContractHash, ContractPackageHash) {
        let mut meta = BTreeMap::new();
        meta.insert("origin".to_string(), "synth".to_string());

        let synth_args = runtime_args! {
            "name" => "synth",
            "contract_name" => "synth",
            "symbol" => "synth",
            "meta" => meta,
            "admin" => Key::Account(*admin)
        };
        let auction_code = PathBuf::from("synth-contract.wasm");
        deploy(
            builder,
            admin,
            &DeploySource::Code(auction_code),
            synth_args,
            true,
            None,
        );

        let contract_hash = query(
            builder,
            Key::Account(*admin),
            &["synth_contract_hash_wrapped".to_string()],
        );
        let contract_package = query(
            builder,
            Key::Account(*admin),
            &["synth_package_hash_wrapped".to_string()],
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
            "kyc_package_hash" => Key::Hash(kyc_package_hash.value()),
            "contract_name" => "NFT".to_string()
        };
        let nft_code = PathBuf::from("nft-contract.wasm");
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

    pub fn deploy_auction(
        wasm: String,
        builder: &mut InMemoryWasmTestBuilder,
        admin: &AccountHash,
        auction_args: RuntimeArgs,
    ) -> (ContractHash, ContractPackageHash) {
        let auction_code = PathBuf::from(wasm);
        let deploy_code = DeploySource::Code(auction_code);
        deploy(
            builder,
            admin,
            &deploy_code,
            auction_args,
            true,
            None,
        );

        let contract_hash: ContractHash = query(
            builder,
            Key::Account(*admin),
            &["test_auction_contract_hash_wrapped".to_string()],
        );
        let contract_package: ContractPackageHash = query(
            builder,
            Key::Account(*admin),
            &["test_auction_contract_package_hash_wrapped".to_string()],
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
        token_id: &str,
        token_meta: &BTreeMap<String, String>,
        sender: &AccountHash,
        commissions: BTreeMap<String, String>,
    ) {
        let args = runtime_args! {
            "recipient" => *recipient,
            "token_ids" => Some(vec![token_id.to_string()]),
            "token_metas" => vec![token_meta.clone()],
            "token_commissions" => vec![commissions],
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

    pub fn add_synth(
        builder: &mut InMemoryWasmTestBuilder,
        synth_package: &ContractPackageHash,
        admin: &AccountHash,
        recipient: &AccountHash,
    ) {
        let mut token_meta = BTreeMap::new();
        token_meta.insert("status".to_string(), "active".to_string());
        token_meta.insert("limit".to_string(), "100000".to_string());

        let args = runtime_args! {
            "recipient" => Key::Account(*recipient),
            "token_id" => Some(recipient.to_string()),
            "token_meta" => token_meta,
        };

        deploy(
            builder,
            admin,
            &DeploySource::ByPackageHash {
                package_hash: *synth_package,
                method: "mint".to_string(),
            },
            args,
            true,
            None,
        );
    }

    pub fn bid(&mut self, bidder: &AccountHash, bid: U512, block_time: u64) {
        let session_code = PathBuf::from("delta-bid-purse.wasm");
        deploy(
            &mut self.builder,
            bidder,
            &DeploySource::Code(session_code),
            runtime_args! {
                "amount" => bid,
                "purse_name" => "my_auction_purse",
                "auction_contract" => self.auction_contract.0
            },
            true,
            Some(block_time),
        );
    }

    pub fn synthetic_bid(&mut self, caller: &AccountHash, bidder: &AccountHash, bid: U512, time: u64) {
        self.call(caller, "synthetic_bid", runtime_args! {
            "bidder" => Key::Account(bidder.clone()),
            "bid" => bid
        }, time)
    }

    pub fn cancel_auction(&mut self, caller: &AccountHash, time: u64) {
        self.call(caller, "cancel_auction", runtime_args! {}, time)
    }

    pub fn approve(&mut self, caller: &AccountHash, time: u64) {
        self.call(caller, "approve", runtime_args! {}, time)
    }

    pub fn reject(&mut self, caller: &AccountHash, time: u64) {
        self.call(caller, "reject", runtime_args! {}, time)
    }

    /// Observers
    pub fn is_live(&self) -> bool {
        let status: u8 = self.query_auction_contract(keys::STATUS);
        status == casper_private_auction_core::data::AUCTION_LIVE
    }

    pub fn is_cancelled(&self) -> bool {
        let status: u8 = self.query_auction_contract(keys::STATUS);
        status == casper_private_auction_core::data::AUCTION_CANCELLED
    }

    pub fn is_pending_settle(&self) -> bool {
        let status: u8 = self.query_auction_contract(keys::STATUS);
        status == casper_private_auction_core::data::AUCTION_PENDING_SETTLE
    }

    pub fn is_rejected(&self) -> bool {
        let status: u8 = self.query_auction_contract(keys::STATUS);
        status == casper_private_auction_core::data::AUCTION_REJECTED
    }

    pub fn is_settled(&self) -> bool {
        let status: u8 = self.query_auction_contract(keys::STATUS);
        status == casper_private_auction_core::data::AUCTION_SETTLED
    }

    pub fn get_end(&self) -> u64 {
        self.query_auction_contract(keys::END)
    }

    pub fn get_current_winner(&self) -> (Option<AccountHash>, Option<(U512, bool)>) {
        let winner: Option<AccountHash> = self.query_auction_contract(keys::CURRENT_WINNER);
        let bid: Option<(U512, bool)> = self.query_auction_contract(keys::WINNING_BID);
        (winner, bid)
    }

    pub fn get_event(&self, index: u32) -> BTreeMap<String, String> {
        self.query_dictionary_value(
            Key::Hash(self.auction_contract.0.value()),
            "auction_events",
            index.to_string(),
        )
            .unwrap()
    }

    pub fn get_events(&self) -> Vec<BTreeMap<String, String>> {
        let mut events = Vec::new();
        for i in 0..self.get_events_count() {
            events.push(self.get_event( i));
        }
        events
    }

    pub fn get_events_count(&self) -> u32 {
        self.query_auction_contract("auction_events_count")
    }

    /// Wrapper function for calling an entrypoint on the contract with the access rights of the deployer.
    pub fn call(&mut self, caller: &AccountHash, method: &str, args: RuntimeArgs, time: u64) {
        deploy(
            &mut self.builder,
            caller,
            &DeploySource::ByPackageHash {
                package_hash: self.auction_contract.1,
                method: method.to_string(),
            },
            args,
            true,
            Some(time),
        );
    }

    fn query_dictionary_value<T: CLTyped + FromBytes>(
        &self,
        base: Key,
        dict_name: &str,
        key: String,
    ) -> Option<T> {
        query_dictionary_item(&self.builder, base, Some(dict_name.to_string()), key)
            .expect("should be stored value.")
            .as_cl_value()
            .expect("should be cl value.")
            .clone()
            .into_t()
            .expect("Wrong type in query result.")
    }

    fn query_auction_contract<T: CLTyped + FromBytes>(&self, name: &str) -> T {
        query(
            &self.builder,
            Key::Account(self.accounts.0),
            &[
                "test_auction_contract_hash".to_string(),
                name.to_string(),
            ],
        )
    }

    /// Getter function for the balance of an account.
    pub fn get_account_balance(&self, account_key: &AccountHash) -> U512 {
        let account = self
            .builder
            .get_account(*account_key)
            .expect("should get genesis account");
        self.builder.get_purse_balance(account.main_purse())
    }

    pub fn get_balances(&self) -> (U512, U512, U512, U512, U512, U512) {
        let admin = self.get_account_balance(&self.accounts.0);
        let market = self.get_account_balance(&self.accounts.1);
        let artist = self.get_account_balance(&self.accounts.2);
        let ali = self.get_account_balance(&self.accounts.3);
        let bob = self.get_account_balance(&self.accounts.4);
        let dan = self.get_account_balance(&self.accounts.5);
        (admin, market, artist, ali, bob, dan)
    }
}
