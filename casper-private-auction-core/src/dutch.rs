use casper_contract::{
    contract_api::runtime,
    unwrap_or_revert::UnwrapOrRevert,
};
pub use casper_types::{
    ApiError, bytesrepr::FromBytes, CLTyped, ContractHash, contracts::NamedKeys,
    Key, runtime_args, RuntimeArgs, system::CallStackElement, U512, URef,
};
pub use casper_types::bytesrepr::ToBytes;

use crate::{
    data::AuctionData,
    events::{AuctionEvent, emit},
};
use crate::auction::Auction;
use crate::data::{AUCTION_CANCELLED, AUCTION_PENDING_SETTLE, AUCTION_SETTLED};
use crate::error::AuctionError;

pub struct DutchAuction;

impl DutchAuction {
    /**
     * Place bid on the auction - if valid, wins the auction
     */
    pub fn bid(account: Key, bid: U512, bidder_purse: Option<URef>) {
        // Get computed current price
        let current_price = AuctionData::current_price();
        if bid < current_price {
            runtime::revert(AuctionError::BidTooLow);
        }

        let bidder = account.into_account()
            .unwrap_or_revert_with(AuctionError::KeyNotAccount);

        // Save the bid
        let mut bids = AuctionData::bids();
        let synthetic = bidder_purse.is_none();
        bids.insert(&bidder, bid.clone(), synthetic);
        AuctionData::update_current_winner(Some(bidder), Some(bid), synthetic);

        // If this is not a synthetic bid, move the tokens...
        if bidder_purse.is_some() {
            // Move the funds to the auction purse
            Auction::move_bid_funds(bidder_purse, bid.clone());
            // Settle the auction
            // TODO: can this be optimized to settle from this purse directly?
            Auction::settle(Some(bidder.into()));
            AuctionData::update_status(AUCTION_SETTLED);
            emit(&AuctionEvent::Settled { account: Some(bidder), bid: Some((current_price, false)) })
        } else {
            // Cannot settle auction, however put it into pending settle
            AuctionData::update_status(AUCTION_PENDING_SETTLE);
            emit(&AuctionEvent::PendingSettlement { account: bidder, bid: (current_price, true) })
        }
    }

    /**
     * Cancel the auction
     */
    pub fn cancel() {
        Auction::settle(None);
        AuctionData::update_status(AUCTION_CANCELLED);
        emit(&AuctionEvent::Cancelled { })
    }
}


