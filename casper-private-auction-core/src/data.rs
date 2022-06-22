use casper_contract::{
    contract_api::{
        runtime::{self, revert},
        storage::{self, new_dictionary},
    },
    unwrap_or_revert::UnwrapOrRevert,
};
use casper_types::{bytesrepr::{FromBytes, ToBytes}, runtime_args, CLTyped, ContractPackageHash, RuntimeArgs, URef, HashAddr};

use crate::{bids::Bids, error::AuctionError, events::{emit, AuctionEvent}, keys, utils};
use alloc::{
    collections::{BTreeMap, BTreeSet},
    format,
    string::{String, ToString},
};
use casper_types::{account::AccountHash, contracts::NamedKeys, Key, U512};
use crate::keys::{read_named_key_uref, read_named_key_value, write_named_key_value};
use crate::utils::{string_to_account_hash, string_to_u16};


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

    pub fn update_current_winner(winner: Option<AccountHash>, bid: Option<U512>) {
        write_named_key_value(WINNER, winner);
        write_named_key_value(PRICE, bid);
        emit(&AuctionEvent::SetWinner {
            bidder: winner,
            bid,
        })
    }

    pub fn auction_format() -> u8 {
        read_named_key_value::<u8>(keys::AUCTION_FORMAT)
    }

    pub fn bids() -> Bids {
        Bids::at()
    }

    pub fn is_finalized() -> bool {
        read_named_key_value::<u8>(keys::STATUS) > 0
    }

    pub fn status() -> u8 {
        read_named_key_value::<u8>(keys::STATUS)
    }

    pub fn update_status(status: u8) {
        write_named_key_value(keys::STATUS, status);
    }

    pub fn current_winner() -> (Option<AccountHash>, Option<U512>) {
        (
            read_named_key_value::<Option<AccountHash>>(keys::CURRENT_WINNER),
            read_named_key_value::<Option<U512>>(keys::WINNING_BID)
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

    pub fn start_time() -> u64 {
        read_named_key_value::<u64>(keys::START)
    }

    pub fn end_time() -> u64 {
        read_named_key_value::<u64>(keys::END)
    }

    pub fn cancel_time() -> u64 {
        read_named_key_value::<u64>(keys::CANCEL)
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

    pub fn marketplace_data() -> (AccountHash, u16) {
        (
            read_named_key_value(keys::MARKETPLACE_ACCOUNT),
            read_named_key_value(keys::MARKETPLACE_COMMISSION),
        )
    }

    pub fn load_commissions() -> BTreeMap<String, String>{
        runtime::call_versioned_contract(
            token_package_hash.clone(),
            None,
            "token_commission",
            runtime_args! {
                "token_id" => token_id,
                "property" => "".to_string(),
            },
        )
            .unwrap_or_revert(AuctionError::MissingCommissions)
    }

    pub fn compute_commissions() -> BTreeMap<AccountHash, u16> {
        let commissions = Self::load_commissions();

        let mut converted_commissions: BTreeMap<AccountHash, u16> = BTreeMap::new();
        let mut done: BTreeSet<String> = BTreeSet::new();
        let mut share_sum = 0;
        for (key, value) in &commissions {
            let mut split = key.split('_');
            let actor = split
                .next()
                .unwrap_or_revert_with(AuctionError::CommissionActorSplit);
            if done.contains(actor) {
                continue;
            }
            let property = split
                .next()
                .unwrap_or_revert_with(AuctionError::CommissionPropertySplit);
            match property {
                "account" => {
                    let rate = commissions
                        .get(&format!("{}_rate", actor))
                        .unwrap_or_revert_with(AuctionError::MismatchedCommissionAccount);
                    let share_rate = string_to_u16(rate);
                    share_sum += share_rate;
                    converted_commissions.insert(string_to_account_hash(value), share_rate);
                }
                "rate" => {
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

    pub fn kyc_package_hash() -> ContractPackageHash {
        read_named_key_value(keys::KYC_PACKAGE_HASH)
    }

    pub fn is_verified(account: &Key) -> bool {
        runtime::call_versioned_contract::<bool>(
            Self::kyc_package_hash(),
            None,
            "is_kyc_proved",
            runtime_args! {
                // "account" => Key::Account(runtime::get_caller()),
                "account" => account,
                "index" => Option::<casper_types::U256>::None
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
}
