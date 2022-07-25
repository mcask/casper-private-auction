use casper_types::{
    account::AccountHash, U512
};

use crate::auction::{AuctionContract, BaseAuctionArgs};
use crate::swap_args::AuctionArgBuilder;

pub struct SwapAuctionContract {
    pub contract: AuctionContract,
}

impl SwapAuctionContract {

    pub fn deploy_with_default_args(start_time: u64) -> Self {
        let mut auction_args = AuctionArgBuilder::default();
        auction_args.set_start_time(start_time);
        auction_args.set_end_time(start_time + 5000);
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

    pub fn cancel_auction(&mut self, caller: &AccountHash, time: u64) {
        self.contract.cancel_auction(caller, time)
    }

    pub fn approve(&mut self, caller: &AccountHash, time: u64) {
        self.contract.approve(caller, time)
    }

    pub fn reject(&mut self, caller: &AccountHash, time: u64) {
        self.contract.reject(caller, time)
    }

}
