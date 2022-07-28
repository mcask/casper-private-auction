use casper_contract::{
    contract_api::{
        runtime::{self, revert},
    },
    unwrap_or_revert::UnwrapOrRevert,
};
use casper_types::{runtime_args, ContractPackageHash, RuntimeArgs, URef};

use crate::{bids::Bids, error::AuctionError, keys};
use alloc::{
    collections::{BTreeMap, BTreeSet},
    format,
    string::{String, ToString},
};
use casper_types::{account::AccountHash, Key, U512};
use casper_types::system::CallStackElement;
use crate::keys::{CURRENT_WINNER, read_named_key_uref, read_named_key_value, WINNING_BID, write_named_key_value};
use crate::utils::{string_to_account_hash, string_to_u16};

// Auction status
pub const AUCTION_LIVE: u8 = 0;
pub const AUCTION_CANCELLED: u8 = 1;
pub const AUCTION_PENDING_SETTLE: u8 = 2;
pub const AUCTION_REJECTED: u8 = 3;
pub const AUCTION_SETTLED: u8 = 4;

const ACCOUNT_TAG: &str = "account";
const RATE_TAG: &str = "rate";

pub struct AuctionData;

impl AuctionData {
    pub fn token_owner() -> Key {
        read_named_key_value::<Key>(keys::OWNER)
    }

    pub fn token_package_hash() -> ContractPackageHash {
        read_named_key_value::<ContractPackageHash>(keys::TOKEN_PACKAGE_HASH)
    }

    pub fn token_id() -> String {
        read_named_key_value::<String>(keys::TOKEN_ID)
    }

    pub fn update_current_winner(winner: Option<AccountHash>, bid: Option<U512>, synthetic: bool) {
        write_named_key_value(CURRENT_WINNER, winner);
        if bid.is_some() {
            write_named_key_value(WINNING_BID, Some((bid.unwrap(), synthetic)));
        } else {
            write_named_key_value(WINNING_BID, Option::<(U512, bool)>::None);
        }
        // emit(&AuctionEvent::SetWinner {
        //     bidder: winner,
        //     bid,
        // })
    }

    // pub fn auction_format() -> u8 {
    //     read_named_key_value::<u8>(keys::AUCTION_FORMAT)
    // }

    pub fn bids() -> Bids {
        Bids::at()
    }

    pub fn is_done() -> bool {
        read_named_key_value::<u8>(keys::STATUS) > AUCTION_LIVE
    }

    pub fn status() -> u8 {
        read_named_key_value::<u8>(keys::STATUS)
    }

    pub fn update_status(status: u8) {
        write_named_key_value(keys::STATUS, status);
    }

    pub fn current_winner() -> (Option<AccountHash>, Option<(U512, bool)>) {
        (
            read_named_key_value::<Option<AccountHash>>(keys::CURRENT_WINNER),
            read_named_key_value::<Option<(U512, bool)>>(keys::WINNING_BID)
        )
    }

    pub fn start_price() -> U512 {
        read_named_key_value::<U512>(keys::START_PRICE)
    }

    pub fn current_price() -> U512 {
        let block_time = u64::from(runtime::get_blocktime());
        let start_price = Self::start_price();
        let end_price = Self::reserve_price();
        let start_time = Self::start_time();
        let end_time = Self::end_time();

        let price_range = start_price - end_price;
        let duration = end_time - start_time;

        let step = price_range / duration;
        let time_passed = block_time - start_time;
        start_price - (step * time_passed)
    }

    pub fn reserve_price() -> U512 {
        read_named_key_value::<U512>(keys::RESERVE_PRICE)
    }

    pub fn swap_price() -> U512 {
        read_named_key_value::<U512>(keys::SWAP_PRICE)
    }

    pub fn start_time() -> u64 {
        read_named_key_value::<u64>(keys::START)
    }

    pub fn end_time() -> u64 {
        read_named_key_value::<u64>(keys::END)
    }

    pub fn cancel_time() -> Option<u64> {
        read_named_key_value::<Option<u64>>(keys::CANCEL)
    }

    pub fn auction_purse() -> URef {
        read_named_key_uref(keys::AUCTION_PURSE)
    }

    pub fn beneficiary_account() -> AccountHash {
        read_named_key_value::<Key>(keys::BENEFICIARY_ACCOUNT)
            .into_account()
            .unwrap_or_revert_with(AuctionError::KeyNotAccount)
    }

    pub fn is_auction_live() -> bool {
        // Check that it's not too late and that the auction isn't finalized
        let start_time = Self::start_time();
        let end_time = Self::end_time();
        let block_time = u64::from(runtime::get_blocktime());

        if block_time < start_time {
            runtime::revert(AuctionError::EarlyBid)
        }
        if block_time >= end_time {
            runtime::revert(AuctionError::LateBid)
        }
        block_time < end_time && block_time >= start_time
    }

    // pub fn set_commissions(commissions: BTreeMap<String, String>) {
    //     write_named_key_value(COMMISSIONS, commissions);
    // }

    // pub fn get_commissions() -> BTreeMap<String, String> {
    //     read_named_key_value(COMMISSIONS)
    // }

    pub fn bidder_count_cap() -> Option<u64> {
        read_named_key_value(keys::BIDDER_NUMBER_CAP)
    }

    pub fn minimum_bid_step() -> Option<U512> {
        read_named_key_value(keys::MINIMUM_BID_STEP)
    }

    pub fn marketplace_data() -> (AccountHash, u32) {
        (
            read_named_key_value(keys::MARKETPLACE_ACCOUNT),
            read_named_key_value(keys::MARKETPLACE_COMMISSION),
        )
    }

    pub fn load_commissions(token_id: &String, token_package_hash: &ContractPackageHash) -> Option<BTreeMap<String, String>>{
        runtime::call_versioned_contract::<Option<BTreeMap<String, String>>>(
            token_package_hash.clone(),
            None,
            "token_commission",
            runtime_args! {
                "token_id" => token_id.to_string(),
            },
        )
    }

    pub fn compute_commissions() -> BTreeMap<AccountHash, u16> {
        let token_id = Self::token_id();
        let token_package_hash = Self::token_package_hash();
        let commissions = Self::load_commissions(&token_id, &token_package_hash)
            .unwrap_or_revert_with(AuctionError::MissingCommissions);

        let mut converted_commissions: BTreeMap<AccountHash, u16> = BTreeMap::new();
        let mut done: BTreeSet<String> = BTreeSet::new();
        let mut share_sum = 0;
        for (key, value) in &commissions {
            let mut split = key.split('_');
            let actor = split
                .next()
                .unwrap_or_revert_with(AuctionError::CommissionActorSplit)
                .to_string();
            if done.contains(&actor) {
                continue;
            }
            let property = split
                .next()
                .unwrap_or_revert_with(AuctionError::CommissionPropertySplit)
                .to_string();
            match property.as_ref() {
                ACCOUNT_TAG => {
                    let rate = commissions
                        .get(&format!("{}_rate", actor))
                        .unwrap_or_revert_with(AuctionError::MismatchedCommissionAccount);
                    let share_rate = string_to_u16(rate);
                    share_sum += share_rate;
                    converted_commissions.insert(string_to_account_hash(value), share_rate);
                }
                RATE_TAG => {
                    let account = commissions
                        .get(&format!("{}_account", actor))
                        .unwrap_or_revert_with(AuctionError::MismatchedCommissionRate);
                    let share_rate = string_to_u16(value);
                    share_sum += share_rate;
                    converted_commissions.insert(string_to_account_hash(account), share_rate);
                }
                _ => revert(AuctionError::InvalidcommissionProperty),
            }
            done.insert(actor.to_string());
        }
        if share_sum > 1000 {
            revert(AuctionError::CommissionTooManyShares)
        }
        converted_commissions
    }

    pub fn kyc_package_hash() -> Option<ContractPackageHash> {
        read_named_key_value(keys::KYC_PACKAGE_HASH)
    }

    pub fn synth_package_hash() -> Option<ContractPackageHash> {
        read_named_key_value(keys::SYNTHETIC_PACKAGE_HASH)
    }

    pub fn is_verified(account: &Key) -> bool {
        let contract_package_hash = Self::kyc_package_hash()
            .unwrap_or_revert_with(AuctionError::KYCError);
        runtime::call_versioned_contract::<bool>(
            contract_package_hash,
            None,
            "is_kyc_proved",
            runtime_args! {
                // "account" => Key::Account(runtime::get_caller()),
                "account" => account.clone(),
                "index" => Option::<casper_types::U256>::None
            },
        )
    }

    pub fn is_allowed(account: &Key, amount: &U512) -> bool {
        let contract_package_hash = Self::synth_package_hash()
            .unwrap_or_revert_with(AuctionError::SyntheticBidNotAllowed);
        runtime::call_versioned_contract::<bool>(
            contract_package_hash,
            None,
            "is_allowed",
            runtime_args! {
                "account" => account.clone(),
                "index" => Option::<casper_types::U256>::None,
                "amount" => amount.clone(),
            },
        )
    }

    pub fn is_enabled(account: &Key) -> bool {
        let contract_package_hash = Self::synth_package_hash()
            .unwrap_or_revert_with(AuctionError::SyntheticBidNotAllowed);
        runtime::call_versioned_contract::<bool>(
            contract_package_hash,
            None,
            "is_enabled",
            runtime_args! {
                "account" => account.clone(),
                "index" => Option::<casper_types::U256>::None,
            },
        )
    }

    pub fn extend_auction() {
        let end: u64 = read_named_key_value::<u64>(keys::END);
        let now: u64 = u64::from(runtime::get_blocktime());
        if now < end {
            let diff: u64 = end - now;
            if let Some(increment) = read_named_key_value::<Option<u64>>(keys::AUCTION_TIMER_EXTENSION) {
                if diff <= increment {
                    write_named_key_value(keys::END, AuctionData::end_time() + increment);
                }
            }
        }
    }

    pub fn current_caller() -> Key {
        Key::Account(runtime::get_caller())
    }

    pub fn current_bidder() -> Key {
        // Figure out who is trying to bid and what their bid is
        let call_stack = runtime::get_call_stack();
        if let Some(CallStackElement::Session { account_hash }) = call_stack.first() {
            Key::Account(*account_hash)
        } else {
            runtime::revert(AuctionError::InvalidCaller)
        }
    }
}
