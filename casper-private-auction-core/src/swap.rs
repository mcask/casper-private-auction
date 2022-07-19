use casper_contract::unwrap_or_revert::UnwrapOrRevert;
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

pub struct Swap;

impl Swap {
    /**
     * Hit the swap
     */
    pub fn bid(account: Key, bidder_purse: Option<URef>) {
        // Get computed current price
        let swap_price = AuctionData::swap_price();
        let bidder = account.into_account()
            .unwrap_or_revert_with(AuctionError::KeyNotAccount);

        // Save the price
        let mut bids = AuctionData::bids();
        let synthetic = bidder_purse.is_none();
        bids.insert(&bidder, swap_price.clone(), synthetic);
        AuctionData::update_current_winner(Some(bidder), Some(swap_price), synthetic);

        // If this is not a synthetic bid, move the tokens...
        if !synthetic {
            // Move the funds to the auction purse
            Auction::move_bid_funds(bidder_purse, swap_price.clone());
            // Settle the auction
            // TODO: can this be optimized to settle from this purse directly?
            Auction::settle(Some(bidder.into()));
            AuctionData::update_status(AUCTION_SETTLED);
            emit(&AuctionEvent::Settled { account: Some(bidder), bid: Some((swap_price, false)) })
        } else {
            // Cannot settle auction, however put it into pending settle
            AuctionData::update_status(AUCTION_PENDING_SETTLE);
            emit(&AuctionEvent::PendingSettlement { account: bidder, bid: (swap_price, true) })
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


