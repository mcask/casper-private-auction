#![no_std]
#![no_main]

extern crate alloc;

use alloc::{format, string::String, vec};

use casper_contract::{
    contract_api::{
        runtime,
        storage, system,
    },
    unwrap_or_revert::UnwrapOrRevert,
};
use casper_types::{ApiError, CLType, ContractPackageHash, EntryPoint, EntryPointAccess, EntryPoints, EntryPointType, Key, Parameter, runtime_args, RuntimeArgs, U512, URef};

use casper_private_auction_core::{accounts, auction::Auction, bids::Bids, constructors, functions, keys};
use casper_private_auction_core::data::AuctionData;
use casper_private_auction_core::english::EnglishAuction;
use casper_private_auction_core::error::AuctionError;

#[no_mangle]
pub extern "C" fn bid() {
    // Standard bids have to be done via session code
    if runtime::get_call_stack().len() != 2 {
        runtime::revert(AuctionError::DisallowedMiddleware);
    }

    Auction::check_valid();

    // Ensure the purse is configured correctly
    let bidder_purse = runtime::get_named_arg::<URef>(keys::BID_PURSE);
    if !bidder_purse.is_writeable() || !bidder_purse.is_readable() {
        runtime::revert(AuctionError::BidderPurseBadPermission)
    }

    // Get the caller from the stack
    let account = AuctionData::current_bidder();
    Auction::verify(&account);

    // Place the bid
    let bid = runtime::get_named_arg::<U512>(keys::BID);
    EnglishAuction::bid(account, bid, Some(bidder_purse));
}

#[no_mangle]
pub extern "C" fn synthetic_bid() {
    // Only admin is allowed to call this
    Auction::check_admin();

    // Ensure auction is still valid
    Auction::check_valid();

    // Check the user is allowed to bid synthetically..
    let account = runtime::get_named_arg::<Key>(keys::BIDDER);
    let bid = runtime::get_named_arg::<U512>(keys::BID);
    Auction::synth_allowed(&account,&bid);

    EnglishAuction::bid(account, bid, Option::<URef>::None);
}

#[no_mangle]
pub extern "C" fn cancel_bid() {
    // This checks we are within cancel time
    EnglishAuction::check_valid();

    // Get the caller from the environment
    let account = AuctionData::current_caller();
    Auction::verify(&account);

    EnglishAuction::cancel_bid(account);
}

#[no_mangle]
pub extern "C" fn cancel_synthetic_bid() {
    // Only admin is allowed to call this
    Auction::check_admin();
    // This checks we are within cancel time
    EnglishAuction::check_valid();

    // All the details are passed in
    let account = runtime::get_named_arg::<Key>(keys::BIDDER);
    Auction::synth_enabled(&account);

    EnglishAuction::cancel_bid(account);
}

#[no_mangle]
pub extern "C" fn cancel_auction() {
    Auction::check_valid();

    // Only owner is allowed to cancel
    Auction::check_owner();

    EnglishAuction::cancel();
}

#[no_mangle]
pub extern "C" fn finalize() {
    // Only owner is allowed to finalize
    Auction::check_owner();

    EnglishAuction::finalize(true);
}

#[no_mangle]
pub extern "C" fn approve() {
    Auction::approve();
}

#[no_mangle]
pub extern "C" fn reject() {
    Auction::reject();
}

#[no_mangle]
pub extern "C" fn init() {
    if runtime::get_key(keys::AUCTION_PURSE).is_none() {
        let purse = system::create_purse();
        runtime::put_key(keys::AUCTION_PURSE, purse.into());
        Bids::init();
    }
}

pub fn get_entry_points() -> EntryPoints {
    let mut entry_points = EntryPoints::new();

    entry_points.add_entry_point(EntryPoint::new(
        functions::BID,
        vec![
            Parameter::new(keys::BID, CLType::U512),
            Parameter::new(keys::BID_PURSE, CLType::URef),
        ],
        CLType::Unit,
        EntryPointAccess::Public,
        EntryPointType::Contract,
    ));

    entry_points.add_entry_point(EntryPoint::new(
        functions::SYNTHETIC_BID,
        vec![
            Parameter::new(keys::BID, CLType::U512),
            Parameter::new(keys::BIDDER, CLType::Key),
        ],
        CLType::Unit,
        EntryPointAccess::Public,
        EntryPointType::Contract,
    ));

    entry_points.add_entry_point(EntryPoint::new(
        functions::CANCEL_BID,
        vec![],
        CLType::Unit,
        EntryPointAccess::Public,
        EntryPointType::Contract,
    ));

    entry_points.add_entry_point(EntryPoint::new(
        functions::CANCEL_SYNTHETIC_BID,
        vec![
            Parameter::new(keys::BIDDER, CLType::Key),
        ],
        CLType::Unit,
        EntryPointAccess::Public,
        EntryPointType::Contract,
    ));

    entry_points.add_entry_point(EntryPoint::new(
        functions::CANCEL_AUCTION,
        vec![],
        CLType::Unit,
        EntryPointAccess::Public,
        EntryPointType::Contract,
    ));

    entry_points.add_entry_point(EntryPoint::new(
        functions::FINALIZE,
        vec![],
        CLType::Unit,
        EntryPointAccess::Public,
        EntryPointType::Contract,
    ));

    entry_points.add_entry_point(EntryPoint::new(
        functions::APPROVE,
        vec![],
        CLType::Unit,
        EntryPointAccess::Public,
        EntryPointType::Contract,
    ));

    entry_points.add_entry_point(EntryPoint::new(
        functions::REJECT,
        vec![],
        CLType::Unit,
        EntryPointAccess::Public,
        EntryPointType::Contract,
    ));

    entry_points.add_entry_point(EntryPoint::new(
        functions::INIT,
        vec![],
        CLType::Unit,
        EntryPointAccess::Public,
        EntryPointType::Contract,
    ));

    entry_points
}

#[no_mangle]
pub extern "C" fn call() {
    let entry_points = get_entry_points();
    let auction_named_keys = constructors::create_english_auction_named_keys(
        Option::Some(accounts::MARKETPLACE_ACCOUNT.into()),
        Option::Some(accounts::MARKETPLACE_COMMISSION.into()),
    );
    let auction_desig: String = runtime::get_named_arg(keys::NAME);
    let (auction_hash, _) = storage::new_locked_contract(
        entry_points,
        Some(auction_named_keys),
        Some(format!("{}_{}", auction_desig, keys::AUCTION_CONTRACT_HASH)),
        Some(format!("{}_{}", auction_desig, keys::AUCTION_ACCESS_TOKEN)),
    );
    let auction_key = Key::Hash(auction_hash.value());
    runtime::put_key(
        &format!("{}_auction_contract_hash", auction_desig),
        auction_key,
    );
    runtime::put_key(
        &format!("{}_auction_contract_hash_wrapped", auction_desig),
        storage::new_uref(auction_hash).into(),
    );

    // Create purse in the contract's context
    runtime::call_contract::<()>(auction_hash, functions::INIT, runtime_args! {});

    // Hash of the NFT contract put up for auction
    let token_contract_hash = ContractPackageHash::new(
        runtime::get_named_arg::<Key>(keys::TOKEN_PACKAGE_HASH)
            .into_hash()
            .unwrap_or_revert_with(ApiError::User(200)),
    );
    // Transfer the NFT ownership to the auction
    let token_ids = vec![runtime::get_named_arg::<String>(keys::TOKEN_ID)];

    let auction_contract_package_hash = runtime::get_key(&format!(
        "{}_{}",
        auction_desig,
        keys::AUCTION_CONTRACT_HASH
    ))
    .unwrap_or_revert_with(ApiError::User(201));
    runtime::put_key(
        &format!("{}_auction_contract_package_hash_wrapped", auction_desig),
        storage::new_uref(ContractPackageHash::new(
            auction_contract_package_hash
                .into_hash()
                .unwrap_or_revert_with(ApiError::User(202)),
        ))
        .into(),
    );
    runtime::call_versioned_contract::<()>(
        token_contract_hash,
        None,
        "transfer",
        runtime_args! {
            "sender" => Key::Account(runtime::get_caller()),
            "recipient" => auction_contract_package_hash,
            "token_ids" => token_ids,
        },
    );
}
