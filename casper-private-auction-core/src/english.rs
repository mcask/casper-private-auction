use casper_contract::{
    contract_api::{
        runtime,
        system,
    },
    unwrap_or_revert::UnwrapOrRevert,
};
use casper_types::account::AccountHash;
pub use casper_types::bytesrepr::ToBytes;
pub use casper_types::{
    bytesrepr::FromBytes, contracts::NamedKeys, runtime_args, system::CallStackElement, ApiError,
    CLTyped, ContractHash, Key, RuntimeArgs, URef, U512,
};

use crate::error::AuctionError;
use crate::{
    data::AuctionData,
    events::{emit, AuctionEvent},
};
use crate::auction::Auction;
use crate::data::{AUCTION_CANCELLED, AUCTION_PENDING_SETTLE, AUCTION_SETTLED};

pub struct EnglishAuction;

impl EnglishAuction {

    // Add a bid to the bid set
    fn add_bid(bidder: &AccountHash, bidder_purse: Option<URef>, new_bid: &U512) {
        let auction_purse = AuctionData::auction_purse();
        // Check the purse is good
        if !auction_purse.is_addable() {
            runtime::revert(AuctionError::AuctionPurseNotAddable)
        }

        // Get the existing bid, if any
        let mut bids = AuctionData::bids();

        // If this is a new bidder and we've hit the cap, release the last bidder
        let existing_bidder = bids.get(&bidder);
        if existing_bidder.is_none() {
            if let Some(bidder_cap) = AuctionData::bidder_count_cap() {
                if bidder_cap <= bids.len() {
                    if let Some((lowest_bidder, lowest_bid)) = bids.get_lowest_bid(new_bid) {
                        bids.remove_by_key(&lowest_bidder);
                        // If the bid was not synthetic, then return it
                        if !lowest_bid.1 {
                            system::transfer_from_purse_to_account(
                                auction_purse,
                                lowest_bidder,
                                lowest_bid.0,
                                None,
                            )
                                .unwrap_or_revert_with(AuctionError::BidReturnLowest);
                        }
                    }
                }
            }
        }
        // Compute the new bid amount
        let bid_amount = if let Some(current_bid) = existing_bidder {
            if *new_bid <= current_bid.0 {
                runtime::revert(AuctionError::NewBidLower)
            }
            *new_bid - current_bid.0
        } else {
            *new_bid
        };
        let synthetic = bidder_purse.is_none();
        if !synthetic {
            system::transfer_from_purse_to_purse(bidder_purse.unwrap(), auction_purse, bid_amount, None)
                .unwrap_or_revert_with(AuctionError::TransferBidToAuction);
        }

        if existing_bidder.is_none() {
            bids.insert(&bidder, *new_bid, synthetic);
        } else {
            bids.replace(&bidder, *new_bid, synthetic);
        }
    }

    /**
     * Specialised check before cancellation
     */
    pub fn check_valid() {
        Auction::check_valid();

        // In addition - check we are within the cancel time
        let cancel_time = AuctionData::cancel_time();
        if cancel_time.is_some() {
            if u64::from(runtime::get_blocktime()) >= cancel_time.unwrap() {
                runtime::revert(AuctionError::LateCancellation)
            }
        }
    }

    /**
     * Place bid on the auction - if valid, becomes the new best price in the auction
     */
    pub fn bid(account: Key, bid: U512, bidder_purse: Option<URef>) {
        let bidder = account.into_account().unwrap_or_revert_with(AuctionError::KeyNotAccount);

        if bid < AuctionData::reserve_price() {
            runtime::revert(AuctionError::BidBelowReserve);
        }
        let (_, winning_bid) = AuctionData::current_winner();
        // If there is a winning bid, ensure this bid is above that, else reject early..
        if winning_bid.is_some() {
            let (wp, _) = winning_bid.unwrap();
            if bid <= wp {
                runtime::revert(AuctionError::BidTooLow);
            } else {
                let min_step = AuctionData::minimum_bid_step().unwrap_or_default();
                let bid_delta = bid - wp;
                if bid_delta < min_step {
                    runtime::revert(AuctionError::BidTooLow);
                }
            }
        }
        let synthetic = bidder_purse.is_none();
        // Save the bid
        Self::add_bid(&bidder, bidder_purse, &bid);
        // Update the current winner
        AuctionData::update_current_winner(Some(bidder), Some(bid), synthetic);
        // See if we need to extend the auction
        AuctionData::extend_auction();

        emit(&AuctionEvent::Bid { account: bidder, bid, synthetic })
    }

    /**
     * Cancel a bid
     */
    pub fn cancel_bid(account: Key) {
        let mut bids = AuctionData::bids();
        let bidder = account.into_account().unwrap_or_revert_with(AuctionError::KeyNotAccount);
        match bids.get(&bidder) {
            Some(current_bid) => {
                if !current_bid.1 {
                    system::transfer_from_purse_to_account(
                        AuctionData::auction_purse(),
                        bidder.into(),
                        current_bid.0,
                        None,
                    )
                        .unwrap_or_revert_with(AuctionError::AuctionCancelReturnBid);
                }
                bids.remove_by_key(&bidder);
                let (new_winner, new_bid, new_synth) = bids.max_by_key();
                AuctionData::update_current_winner(new_winner, new_bid, new_synth);

                emit(&AuctionEvent::BidCancelled { account: bidder })
            }
            None => runtime::revert(AuctionError::NoBid),
        }
    }

    /**
     * Finalize the auction if possible - only callable by owner
     */
    pub fn finalize(time_check: bool) {
        // Get finalization and check if we're done
        if AuctionData::is_done() {
            runtime::revert(AuctionError::AlreadyFinal)
        };

        // Cannot finalize before the end of the auction
        if time_check && u64::from(runtime::get_blocktime()) < AuctionData::end_time() {
            runtime::revert(AuctionError::EarlyFinalize)
        }

        // See if there is a winner
        match AuctionData::current_winner() {
            (Some(bidder), Some(bid)) => {
                // Synthetic bid - put it in pending settle state
                if bid.1 {
                    AuctionData::update_status(AUCTION_PENDING_SETTLE);
                    emit(&AuctionEvent::PendingSettlement { account: bidder, bid })
                } else {
                    Auction::settle(Some(bidder.into()));
                    AuctionData::update_status(AUCTION_SETTLED);
                    emit(&AuctionEvent::Settled { account: Some(bidder), bid: Some(bid) })
                }
            }
            _ => {
                Auction::settle(None);
                AuctionData::update_status(AUCTION_SETTLED);
                emit(&AuctionEvent::Settled { account: None, bid: None });
            }
        };
    }

    /**
     * Cancel the auction only if there are no bids
     */
    pub fn cancel() {
        // If we have a current winner, then this auction cannot be cancelled
        let (winner, winning_bid) = AuctionData::current_winner();
        if winner.is_none() && winning_bid.is_none() {
            Auction::settle(None);
            AuctionData::update_status(AUCTION_CANCELLED);
            emit(&AuctionEvent::Cancelled { });
            return
        }
        runtime::revert(AuctionError::CannotCancelAuction);
    }
}


