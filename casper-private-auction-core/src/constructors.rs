use alloc::string::String;
use casper_contract::contract_api::runtime;
use casper_contract::contract_api::storage;
use casper_contract::contract_api::runtime::revert;
use casper_contract::unwrap_or_revert::UnwrapOrRevert;
use casper_types::{ContractPackageHash, Key, U512, account::AccountHash};
use casper_types::contracts::NamedKeys;
use crate::{AuctionError, keys, utils};
use crate::data::{AuctionData};

macro_rules! named_keys {
    ( $( ($name:expr, $value:expr) ),* ) => {
        {
            let mut named_keys = NamedKeys::new();
            $( named_keys.insert($name.into(), storage::new_uref($value).into()); )*
            named_keys
        }
    };
}

fn get_cancellable_times() -> (u64, Option<u64>, u64) {
    let start: u64 = runtime::get_named_arg(keys::START);
    let cancel = runtime::get_named_arg::<Option<u64>>(keys::CANCEL);
    let end: u64 = runtime::get_named_arg(keys::END);

    if cancel.is_some() {
        let cts = cancel.unwrap();
        if u64::from(runtime::get_blocktime()) <= start
            && start <= cts
            && cts <= end
            && start < end
        {
            return (start, cancel, end);
        }
        runtime::revert(AuctionError::InvalidTimes)
    }

    if u64::from(runtime::get_blocktime()) <= start
        && start < end
    {
        return (start, cancel, end);
    }

    runtime::revert(AuctionError::InvalidTimes)
}

fn get_fixed_times() -> (u64, u64) {
    let start: u64 = runtime::get_named_arg(keys::START);
    let end: u64 = runtime::get_named_arg(keys::END);
    if u64::from(runtime::get_blocktime()) <= start
        && start < end
    {
        return (start, end);
    }
    runtime::revert(AuctionError::InvalidTimes)
}

fn validate_commissions(token_id: &String, token_package_hash: &ContractPackageHash) {
    let commissions = AuctionData::load_commissions(&token_id, &token_package_hash)
        .unwrap_or_revert_with(AuctionError::MissingCommissions);

    let mut share_sum = 0;
    for (key, value) in &commissions {
        let mut split = key.split('_');
        let _actor = split
            .next()
            .unwrap_or_revert_with(AuctionError::CommissionActorSplit);
        let property = split
            .next()
            .unwrap_or_revert_with(AuctionError::CommissionPropertySplit);
        match property {
            "account" => {
                utils::string_to_account_hash(value);
            },
            "rate" => {
                share_sum += utils::string_to_u16(value);
            }
            _ => revert(AuctionError::InvalidcommissionProperty),
        }
    }
    if share_sum > 1000 {
        revert(AuctionError::CommissionTooManyShares)
    }
}

fn get_token() -> (Key, Key, String, ContractPackageHash) {
    let token_owner = Key::Account(runtime::get_caller());
    // Get the beneficiary purse
    let beneficiary_account = match runtime::get_named_arg::<Key>(keys::BENEFICIARY_ACCOUNT) {
        key @ Key::Account(_) => key,
        _ => runtime::revert(AuctionError::InvalidBeneficiary),
    };
    let token_id = runtime::get_named_arg::<String>(keys::TOKEN_ID);
    let token_contract_hash = runtime::get_named_arg::<Key>(keys::TOKEN_PACKAGE_HASH)
        .into_hash()
        .unwrap_or_revert_with(AuctionError::MissingTokenPackageHash);
    return (token_owner, beneficiary_account, token_id, ContractPackageHash::from(token_contract_hash));
}

fn get_proxy_contracts() -> (Option<ContractPackageHash>, Option<ContractPackageHash>) {
    let kyc_package_hash = match runtime::get_named_arg::<Key>(keys::KYC_PACKAGE_HASH)
        .into_hash() {
        Some(v) => Some(ContractPackageHash::from(v)),
        None => None,
    };
    let synth_package_hash = match runtime::get_named_arg::<Key>(keys::SYNTHETIC_PACKAGE_HASH)
        .into_hash() {
        Some(v) => Some(ContractPackageHash::from(v)),
        None => None,
    };
    return (kyc_package_hash, synth_package_hash);
}

pub fn create_english_auction_named_keys(marketplace_account: AccountHash, marketplace_commission: u32) -> NamedKeys {
    // Get the token info
    let (token_owner, beneficiary_account, token_id, token_package_hash) = get_token();
    // Validate the commission structure in the NFT
    validate_commissions(&token_id, &token_package_hash);

    // Get the proxy contracts
    let (kyc_package_hash, synth_package_hash) = get_proxy_contracts();

    // Prices
    let reserve_price = runtime::get_named_arg::<U512>(keys::RESERVE_PRICE);
    if reserve_price <= U512::from(1000_u64) {
        runtime::revert(AuctionError::InvalidPrices);
    }
    // Times
    let (start_time, cancellation_time, end_time) = get_cancellable_times();

    // Starting state
    let winning_bid: Option<U512> = None;
    let current_winner: Option<Key> = None;
    let status = 0_u8;

    // Auction properties
    let bidder_count_cap = runtime::get_named_arg::<Option<u64>>(keys::BIDDER_NUMBER_CAP)
        .unwrap_or_else(|| 5);
    let auction_timer_extension = runtime::get_named_arg::<Option<u64>>(keys::AUCTION_TIMER_EXTENSION)
        .unwrap_or_else(|| 5 * 60 * 1000);
    let minimum_bid_step = runtime::get_named_arg::<Option<U512>>(keys::MINIMUM_BID_STEP);

    let mut named_keys = named_keys!(
        (keys::OWNER, token_owner),
        (keys::BENEFICIARY_ACCOUNT, beneficiary_account),
        (keys::TOKEN_PACKAGE_HASH, token_package_hash),
        (keys::KYC_PACKAGE_HASH, kyc_package_hash),
        (keys::SYNTHETIC_PACKAGE_HASH, synth_package_hash),
        (keys::TOKEN_ID, token_id),
        (keys::START, start_time),
        (keys::CANCEL, cancellation_time),
        (keys::END, end_time),
        (keys::RESERVE_PRICE, reserve_price),
        (keys::WINNING_BID, winning_bid),
        (keys::CURRENT_WINNER, current_winner),
        (keys::STATUS, status),
        (keys::EVENTS_COUNT, 0_u32),
        (keys::BIDDER_NUMBER_CAP, bidder_count_cap),
        (keys::AUCTION_TIMER_EXTENSION, auction_timer_extension),
        (keys::MINIMUM_BID_STEP, minimum_bid_step),
        (keys::MARKETPLACE_COMMISSION, marketplace_commission),
        (keys::MARKETPLACE_ACCOUNT, marketplace_account)
    );
    utils::add_empty_dict(&mut named_keys, keys::EVENTS);
    named_keys
}

pub fn create_dutch_auction_named_keys(marketplace_account: AccountHash, marketplace_commission: u32) -> NamedKeys {
    // Get the token info
    let (token_owner, beneficiary_account, token_id, token_package_hash) = get_token();
    // Validate the commission structure in the NFT
    validate_commissions(&token_id, &token_package_hash);

    // Get the proxy contracts
    let (kyc_package_hash, synth_package_hash) = get_proxy_contracts();

    // Prices
    let start_price = runtime::get_named_arg::<U512>(keys::START_PRICE);
    let reserve_price = runtime::get_named_arg::<U512>(keys::RESERVE_PRICE);
    if start_price <= U512::from(1000_u64) || reserve_price <= U512::from(1000_u64) {
        runtime::revert(AuctionError::InvalidPrices);
    }
    if start_price <= reserve_price {
        runtime::revert(AuctionError::InvalidPrices)
    }
    // Times
    let (start_time, end_time) = get_fixed_times();

    // Starting state
    let winning_bid: Option<U512> = None;
    let current_winner: Option<Key> = None;
    let status = 0_u8;

    let mut named_keys = named_keys!(
        (keys::OWNER, token_owner),
        (keys::BENEFICIARY_ACCOUNT, beneficiary_account),
        (keys::TOKEN_PACKAGE_HASH, token_package_hash),
        (keys::KYC_PACKAGE_HASH, kyc_package_hash),
        (keys::SYNTHETIC_PACKAGE_HASH, synth_package_hash),
        (keys::TOKEN_ID, token_id),
        (keys::START, start_time),
        (keys::END, end_time),
        (keys::START_PRICE, start_price),
        (keys::RESERVE_PRICE, reserve_price),
        (keys::WINNING_BID, winning_bid),
        (keys::CURRENT_WINNER, current_winner),
        (keys::STATUS, status),
        (keys::EVENTS_COUNT, 0_u32),
        (keys::MARKETPLACE_COMMISSION, marketplace_commission),
        (keys::MARKETPLACE_ACCOUNT, marketplace_account)
    );
    utils::add_empty_dict(&mut named_keys, keys::EVENTS);
    named_keys
}

pub fn create_swap_named_keys(marketplace_account: AccountHash, marketplace_commission: u32) -> NamedKeys {
    // Get the token info
    let (token_owner, beneficiary_account, token_id, token_package_hash) = get_token();
    // Validate the commission structure in the NFT
    validate_commissions(&token_id, &token_package_hash);
    // Get the proxy contracts
    let (kyc_package_hash, synth_package_hash) = get_proxy_contracts();

    // Prices
    let swap_price = runtime::get_named_arg::<U512>(keys::SWAP_PRICE);
    if swap_price <= U512::from(1000_u64) {
        runtime::revert(AuctionError::InvalidPrices);
    }
    // Times
    let (start_time, end_time) = get_fixed_times();

    // Starting state
    let winning_bid: Option<U512> = None;
    let current_winner: Option<Key> = None;
    let status = 0_u8;

    let mut named_keys = named_keys!(
        (keys::OWNER, token_owner),
        (keys::BENEFICIARY_ACCOUNT, beneficiary_account),
        (keys::TOKEN_PACKAGE_HASH, token_package_hash),
        (keys::KYC_PACKAGE_HASH, kyc_package_hash),
        (keys::SYNTHETIC_PACKAGE_HASH, synth_package_hash),
        (keys::TOKEN_ID, token_id),
        (keys::START, start_time),
        (keys::END, end_time),
        (keys::SWAP_PRICE, swap_price),
        (keys::WINNING_BID, winning_bid),
        (keys::CURRENT_WINNER, current_winner),
        (keys::STATUS, status),
        (keys::EVENTS_COUNT, 0_u32),
        (keys::MARKETPLACE_COMMISSION, marketplace_commission),
        (keys::MARKETPLACE_ACCOUNT, marketplace_account)
    );
    utils::add_empty_dict(&mut named_keys, keys::EVENTS);
    named_keys
}

