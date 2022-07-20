use std::collections::BTreeMap;

use casper_types::{account::AccountHash, runtime_args, RuntimeArgs, U512};

use crate::auction::{AuctionContract, BaseAuctionArgs};
use crate::english_args::AuctionArgBuilder;

pub struct EnglishAuctionContract {
    pub contract: AuctionContract,
}

impl EnglishAuctionContract {

    pub fn deploy_with_default_args(start_time: u64) -> Self {
        let mut auction_args = AuctionArgBuilder::default();
        auction_args.set_start_time(start_time);
        let contract = AuctionContract::deploy(&mut auction_args);
        Self {
            contract
        }
    }

    pub fn deploy(mut auction_args: AuctionArgBuilder) -> Self {
        let contract = AuctionContract::deploy(&mut auction_args);
        Self {
            contract
        }
    }

    pub fn bid(&mut self, bidder: &AccountHash, bid: U512, block_time: u64) {
        self.contract.bid(bidder, bid, block_time)
    }

    pub fn synthetic_bid(&mut self, admin: &AccountHash, bidder: &AccountHash, bid: U512, block_time: u64) {
        self.contract.synthetic_bid(admin, bidder, bid, block_time)
    }

    pub fn cancel_bid(&mut self, bidder: &AccountHash, block_time: u64) {
        self.contract.call(bidder, "cancel_bid", runtime_args! {}, block_time)
    }

    pub fn cancel_synthetic_bid(&mut self, admin: &AccountHash, bidder: &AccountHash, block_time: u64) {
        self.contract.call(admin, "cancel_bid", runtime_args! {
            "bidder" => bidder.clone(),
        }, block_time)
    }

    pub fn cancel_auction(&mut self, caller: &AccountHash, time: u64) {
        self.contract.cancel_auction(caller, time)
    }

    pub fn approve(&mut self, caller: &AccountHash, time: u64) {
        self.contract.approve(caller, time)
    }

    pub fn reject(&mut self, caller: &AccountHash, time: u64) {
        self.contract.reject(caller, time)
    }

    pub fn get_end(&self) -> u64 {
        self.contract.get_end()
    }

    pub fn get_current_winner(&self) -> (Option<AccountHash>, Option<(U512, bool)>) {
        self.contract.get_current_winner()
    }

    pub fn get_event(&self, contract_hash: [u8; 32], index: u32) -> BTreeMap<String, String> {
        self.contract.get_event(contract_hash, index)
    }

    pub fn get_events(&self, contract_hash: [u8; 32]) -> Vec<BTreeMap<String, String>> {
        self.contract.get_events(contract_hash)
    }

    pub fn get_events_count(&self, contract_hash: [u8; 32]) -> u32 {
        self.contract.get_events_count(contract_hash)
    }

    pub fn get_all_accounts_balance(&self) -> (U512, U512, U512) {
        self.contract.get_all_accounts_balance()
    }

    pub fn get_marketplace_balance(&self) -> U512 {
        self.contract.get_marketplace_balance()
    }

    pub fn get_comm_balance(&self) -> U512 {
        self.contract.get_comm_balance()
    }
}
