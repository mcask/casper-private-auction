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
use crate::data::{AUCTION_PENDING_SETTLE, AUCTION_REJECTED, AUCTION_SETTLED};
use crate::keys::MARKETPLACE_ACCOUNT;
use crate::utils::string_to_account_hash;

pub struct Auction;

impl Auction {

    // Check if auction is still live
    pub fn check_valid() {
        if !AuctionData::is_auction_live() || AuctionData::is_done() {
            runtime::revert(AuctionError::BadState)
        }
    }

    // Check the given account is verified
    pub fn verify(account: &Key) {
        if !AuctionData::is_verified(account) {
            runtime::revert(AuctionError::KYCError);
        }
    }

    // Check the given account is allowed to make a synthetic bid to this amount
    pub fn synth_allowed(account: &Key, amount: &U512) {
        if !AuctionData::is_allowed(account, amount) {
            runtime::revert(AuctionError::SyntheticBidNotAllowed);
        }
    }

    pub fn synth_enabled(account: &Key) {
        if !AuctionData::is_enabled(account) {
            runtime::revert(AuctionError::SyntheticBidNotAllowed);
        }
    }

    pub fn check_owner() {
        if AuctionData::token_owner() != Key::Account(runtime::get_caller()) {
            runtime::revert(AuctionError::InvalidCaller);
        }
    }

    pub fn check_admin() {
        if string_to_account_hash(MARKETPLACE_ACCOUNT) != runtime::get_caller() {
            runtime::revert(AuctionError::InvalidCaller);
        }
    }

    pub fn move_bid_funds(bidder_purse: Option<URef>, bid: U512) {
        let auction_purse = AuctionData::auction_purse();
        let purse = bidder_purse.unwrap();
        if !purse.is_writeable() || !purse.is_readable() {
            runtime::revert(AuctionError::BidderPurseBadPermission)
        }
        if !auction_purse.is_addable() {
            runtime::revert(AuctionError::AuctionPurseNotAddable)
        }
        system::transfer_from_purse_to_purse(purse, auction_purse, bid, None)
            .unwrap_or_revert_with(AuctionError::TransferBidToAuction);
    }
    //
    // fn add_bid(bidder: AccountHash, bidder_purse: URef, new_bid: U512) {
    //     if !AuctionData::is_auction_live() || AuctionData::is_finalized() {
    //         runtime::revert(AuctionError::BadState)
    //     }
    //     if !AuctionData::is_verified() {
    //         runtime::revert(AuctionError::KYCError);
    //     }
    //     // Get the existing bid, if any
    //     let mut bids = AuctionData::bids();
    //     let auction_purse = AuctionData::auction_purse();
    //     if bids.get(&bidder).is_none() {
    //         if let Some(bidder_cap) = AuctionData::bidder_count_cap() {
    //             if bidder_cap <= bids.len() {
    //                 if let Some((lowest_bidder, lowest_bid)) = bids.get_spot(new_bid) {
    //                     bids.remove_by_key(&lowest_bidder);
    //                     system::transfer_from_purse_to_account(
    //                         auction_purse,
    //                         lowest_bidder,
    //                         lowest_bid,
    //                         None,
    //                     )
    //                     .unwrap_or_revert_with(AuctionError::BidReturnLowest);
    //                 }
    //             }
    //         }
    //     }
    //     let bid_amount = if let Some(current_bid) = bids.get(&bidder) {
    //         if new_bid <= current_bid {
    //             runtime::revert(AuctionError::NewBidLower)
    //         }
    //         new_bid - current_bid
    //     } else {
    //         new_bid
    //     };
    //     if !bidder_purse.is_writeable() || !bidder_purse.is_readable() {
    //         runtime::revert(AuctionError::BidderPurseBadPermission)
    //     }
    //     if !auction_purse.is_addable() {
    //         runtime::revert(AuctionError::AuctionPurseNotAddable)
    //     }
    //     system::transfer_from_purse_to_purse(bidder_purse, auction_purse, bid_amount, None)
    //         .unwrap_or_revert_with(AuctionError::TransferBidToAuction);
    //     bids.replace(&bidder, new_bid);
    // }

    // pub fn find_new_winner() -> Option<(AccountHash, (U512, bool))> {
    //     let bids = AuctionData::bids();
    //     let winning_pair = bids.max_by_key();
    //     match winning_pair {
    //         Some((key, bid)) => Some((key, bid)),
    //         _ => None,
    //     }
    // }

    // pub fn get_bidder() -> AccountHash {
    //     // Figure out who is trying to bid and what their bid is
    //     let call_stack = runtime::get_call_stack();
    //     // if call_stack.len() == 2 {runtime::revert(AuctionError::InvalidCallStackLenght)}
    //     if let Some(CallStackElement::Session { account_hash }) = call_stack.first() {
    //         *account_hash
    //     } else {
    //         runtime::revert(AuctionError::InvalidCaller)
    //     }
    // }

    fn transfer_token(recipient: Key) {
        let auction_key: Key = {
            let call_stack = runtime::get_call_stack();
            let caller: CallStackElement = call_stack
                .last()
                .unwrap_or_revert_with(AuctionError::CallStackTooShort)
                .clone();
            match caller {
                CallStackElement::StoredContract {
                    contract_package_hash,
                    contract_hash: _,
                } => Key::Hash(contract_package_hash.value()),
                _ => runtime::revert(AuctionError::InvalidCaller),
            }
        };
        let mut token_ids = alloc::vec::Vec::new();
        token_ids.push(AuctionData::token_id());
        runtime::call_versioned_contract(
            AuctionData::token_package_hash(),
            None,
            "transfer",
            runtime_args! {
              "sender" => auction_key,
              "recipient" => recipient,
              "token_ids" => token_ids,
            },
        )
    }

    /**
     * Handle transferring the token and funds
     */
    pub fn settle(winner: Option<AccountHash>) {
        // If there is a winner, then move the token to the winner
        // else send it back to the owner
        match winner {
            Some(acct) => Self::transfer_token(Key::Account(acct)),
            _ => Self::transfer_token(AuctionData::token_owner()),
        }

        fn return_bids(auction_purse: URef) {
            let mut bids = AuctionData::bids();
            for (bidder, bid) in &bids.to_map() {
                system::transfer_from_purse_to_account(auction_purse, *bidder, bid.0.clone(), None)
                    .unwrap_or_revert_with(AuctionError::AuctionEndReturnBids);
            }
            bids.clear();
        }
        let auction_purse = AuctionData::auction_purse();
        match winner {
            Some(key) => {
                let mut bids = AuctionData::bids();
                match bids.get(&key) {
                    Some(bid) => {
                        // Marketplace share first, then people get money
                        let (marketplace_account, marketplace_commission) =
                            AuctionData::marketplace_data();
                        let market_share = (bid.0 / 1000) * marketplace_commission;
                        system::transfer_from_purse_to_account(
                            auction_purse,
                            marketplace_account,
                            market_share,
                            None,
                        )
                        .unwrap_or_revert_with(AuctionError::TransferMarketPlaceShare);
                        let proceeds = bid.0 - market_share;
                        // Every actor receives x one-thousandth of the winning bid, the surplus goes to the designated beneficiary account.
                        let share_piece = proceeds / 1000;
                        let mut given_as_shares = U512::zero();
                        for (account, share) in AuctionData::compute_commissions() {
                            let actor_share = share_piece * share;
                            if actor_share == U512::from(0_u64) {
                                runtime::revert(AuctionError::BadState);
                            }
                            system::transfer_from_purse_to_account(
                                auction_purse,
                                account,
                                actor_share,
                                None,
                            )
                            .unwrap_or_revert_with(AuctionError::TransferCommissionShare);
                            given_as_shares += actor_share;
                        }
                        system::transfer_from_purse_to_account(
                            auction_purse,
                            AuctionData::beneficiary_account(),
                            proceeds - given_as_shares,
                            None,
                        )
                        .unwrap_or_revert_with(AuctionError::TransferBeneficiaryShare);
                        bids.remove_by_key(&key);
                        return_bids(auction_purse);
                    }
                    // Something went wrong, so better return everyone's money
                    _ => return_bids(auction_purse),
                }
            }
            _ => {
                return_bids(auction_purse);
            }
        }
    }

    pub fn approve() {
        // Only admin is allowed to call this
        Self::check_admin();

        if AuctionData::status() != AUCTION_PENDING_SETTLE {
            runtime::revert(AuctionError::BadState)
        }

        // Get the winner
        let (winner, bid) = AuctionData::current_winner();
        Self::settle(winner.into());
        AuctionData::update_status(AUCTION_SETTLED);
        emit(&AuctionEvent::Settled { account: winner, bid })
    }

    pub fn reject() {
        // Only admin is allowed to call this
        Self::check_admin();

        if AuctionData::status() != AUCTION_PENDING_SETTLE {
            runtime::revert(AuctionError::BadState)
        }

        // Get the winner (who did not settle)
        let (winner, _bid) = AuctionData::current_winner();
        Self::settle(Option::None);
        AuctionData::update_status(AUCTION_REJECTED);
        emit(&AuctionEvent::SettlementRejected { account: winner })
    }

    //
    // fn auction_bid() {
    //     if !AuctionData::is_auction_live() || AuctionData::is_finalized() {
    //         runtime::revert(AuctionError::BadState)
    //     }
    //     if !AuctionData::is_verified() {
    //         runtime::revert(AuctionError::KYCError);
    //     }
    //     if get_call_stack().len() != 2 {
    //         runtime::revert(AuctionError::DisallowedMiddleware);
    //     }
    //     // We do not check times here because we do that in Auction::add_bid
    //     // Figure out who is trying to bid and what their bid is
    //     let bidder = Self::get_bidder();
    //     let bid = runtime::get_named_arg::<U512>(crate::data::BID);
    //     if bid < AuctionData::reserve_price() {
    //         runtime::revert(AuctionError::BidBelowReserve);
    //     }
    //     let bidder_purse = runtime::get_named_arg::<URef>(crate::data::BID_PURSE);
    //     // Adding the bid, doing the purse transfer and resetting the winner if necessary, as well as possibly ending a Dutch auction
    //     let winner = AuctionData::get_winner();
    //     let price = AuctionData::get_price();
    //     if !AuctionData::is_english_format() {
    //         if let (None, None) = (winner, price) {
    //             let current_price = AuctionData::current_price();
    //             if bid < current_price {
    //                 runtime::revert(AuctionError::BidTooLow);
    //             }
    //             Self::add_bid(bidder, bidder_purse, current_price);
    //             AuctionData::update_current_winner(Some(bidder), Some(bid));
    //             Self::auction_finalize(false);
    //         } else {
    //             runtime::revert(AuctionError::BadState);
    //         }
    //     } else {
    //         Self::add_bid(bidder, bidder_purse, bid);
    //         if let (Some(_), Some(current_price)) = (winner, price) {
    //             let min_step = AuctionData::minimum_bid_step().unwrap_or_default();
    //             if bid > current_price && bid - current_price >= min_step {
    //                 AuctionData::update_current_winner(Some(bidder), Some(bid));
    //             } else {
    //                 runtime::revert(AuctionError::BidTooLow)
    //             }
    //         } else if let (None, None) = (winner, price) {
    //             AuctionData::update_current_winner(Some(bidder), Some(bid));
    //         } else {
    //             runtime::revert(AuctionError::BadState)
    //         }
    //     }
    //
    //     AuctionData::extend_auction();
    //     emit(&AuctionEvent::Bid { bidder, bid })
    // }
    //
    // fn auction_cancel_bid() {
    //     let bidder = Self::get_bidder();
    //
    //     if u64::from(runtime::get_blocktime()) < AuctionData::cancel_time() {
    //         let mut bids = AuctionData::bids();
    //
    //         match bids.get(&bidder) {
    //             Some(current_bid) => {
    //                 system::transfer_from_purse_to_account(
    //                     AuctionData::auction_purse(),
    //                     bidder,
    //                     current_bid,
    //                     None,
    //                 )
    //                 .unwrap_or_revert_with(AuctionError::AuctionCancelReturnBid);
    //                 bids.remove_by_key(&bidder);
    //                 match Self::find_new_winner() {
    //                     Some((winner, bid)) => AuctionData::update_current_winner(Some(winner), Some(bid)),
    //                     _ => AuctionData::update_current_winner(None, None),
    //                 }
    //             }
    //             None => runtime::revert(AuctionError::NoBid),
    //         }
    //     } else {
    //         runtime::revert(AuctionError::LateCancellation)
    //     }
    //     emit(&AuctionEvent::BidCancelled { bidder })
    // }

    // fn auction_finalize(time_check: bool) {
    //     // Get finalization and check if we're done
    //     if AuctionData::is_finalized() {
    //         runtime::revert(AuctionError::AlreadyFinal)
    //     };
    //
    //     // We're not finalized, so let's get all the other arguments, as well as time to make sure we're not too early
    //     if time_check && u64::from(runtime::get_blocktime()) < AuctionData::end_time() {
    //         runtime::revert(AuctionError::EarlyFinalize)
    //     }
    //
    //     // TODO: Figure out how to gracefully finalize if the keys are bad
    //     let winner = match (AuctionData::get_price(), AuctionData::get_winner()) {
    //         (Some(winning_bid), Some(winner)) => {
    //             Self::auction_allocate(Some(winner));
    //             Self::auction_transfer(Some(winner));
    //             AuctionData::set_finalized();
    //             Some((winner, winning_bid))
    //         }
    //         _ => {
    //             Self::auction_allocate(None);
    //             Self::auction_transfer(None);
    //             AuctionData::set_finalized();
    //             None
    //         }
    //     };
    //     emit(&AuctionEvent::Finalized { winner })
    // }
    //
    // fn cancel_auction() {
    //     if AuctionData::token_owner() != Key::Account(runtime::get_caller()) {
    //         runtime::revert(AuctionError::InvalidCaller);
    //     }
    //     if !AuctionData::bids().is_empty() && AuctionData::get_winner().is_some() {
    //         runtime::revert(AuctionError::CannotCancelAuction);
    //     }
    //
    //     Self::auction_allocate(None);
    //     Self::auction_transfer(None);
    //     AuctionData::set_finalized();
    // }
}


