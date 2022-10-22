use casper_contract::contract_api::{runtime, storage};
use casper_contract::unwrap_or_revert::UnwrapOrRevert;
use casper_types::{CLTyped, URef};
use casper_types::bytesrepr::{FromBytes, ToBytes};
use crate::AuctionError;

pub const CONTRACT_TYPE: &str = "contract_type";
pub const BID: &str = "bid";
pub const BIDDER: &str = "bidder";
pub const SENDER: &str = "sender";
pub const RECEIVER: &str = "receiver";
pub const OWNER: &str = "token_owner";
pub const BENEFICIARY_ACCOUNT: &str = "beneficiary_account";
pub const AUCTION_PURSE: &str = "auction_purse";
pub const TOKEN_PACKAGE_HASH: &str = "token_package_hash";
pub const TOKEN_ID: &str = "token_id";
pub const START: &str = "start_time";
pub const CANCEL: &str = "cancellation_time";
pub const END: &str = "end_time";
pub const SWAP_PRICE: &str = "swap_price";
pub const RESERVE_PRICE: &str = "reserve_price";
pub const START_PRICE: &str = "starting_price";
pub const WINNING_BID: &str = "winning_bid";
pub const CURRENT_WINNER: &str = "current_winner";
pub const STATUS: &str = "status";
pub const BID_PURSE: &str = "bid_purse";
pub const AUCTION_CONTRACT_HASH: &str = "auction_contract_package_hash";
pub const AUCTION_ACCESS_TOKEN: &str = "auction_access_token";
pub const GIFT_CONTRACT_HASH: &str = "gift_contract_package_hash";
pub const GIFT_ACCESS_TOKEN: &str = "gift_access_token";
pub const EVENTS: &str = "auction_events";
pub const EVENTS_COUNT: &str = "auction_events_count";
pub const COMMISSIONS: &str = "commissions";
pub const KYC_PACKAGE_HASH: &str = "kyc_package_hash";
pub const SYNTHETIC_PACKAGE_HASH: &str = "synth_package_hash";
pub const BIDDER_NUMBER_CAP: &str = "bidder_count_cap";
pub const AUCTION_TIMER_EXTENSION: &str = "auction_timer_extension";
pub const MINIMUM_BID_STEP: &str = "minimum_bid_step";
pub const MARKETPLACE_COMMISSION: &str = "marketplace_commission";
pub const MARKETPLACE_ACCOUNT: &str = "marketplace_account";
pub const NAME: &str = "name";
pub const TOKENS: &str = "gift_tokens";
pub const TOKEN_COUNT: &str = "token_count";

// TODO: This needs A LOT of error handling because we don't want an auction being left in an unrecoverable state if the named keys are bad!
pub fn read_named_key_uref(name: &str) -> URef {
    runtime::get_key(name)
        .unwrap_or_revert_with(AuctionError::CannotReadKey)
        .into_uref()
        .unwrap_or_revert_with(AuctionError::KeyNotUref)
}

// TODO: This needs A LOT of error handling because we don't want an auction being left in an unrecoverable state if the named keys are bad!
pub fn read_named_key_value<T: CLTyped + FromBytes>(name: &str) -> T {
    let uref = read_named_key_uref(name);

    storage::read(uref)
        .unwrap_or_revert_with(AuctionError::CannotReadKey)
        .unwrap_or_revert_with(AuctionError::NamedKeyNotFound)
}

pub fn write_named_key_value<T: CLTyped + ToBytes>(name: &str, value: T) {
    let uref = read_named_key_uref(name);
    storage::write(uref, value);
}