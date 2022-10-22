#![no_std]
#![no_main]

extern crate alloc;

use alloc::{format, string::String, vec};
use alloc::boxed::Box;

use casper_contract::{
    contract_api::{
        runtime,
        storage, system,
    },
    unwrap_or_revert::UnwrapOrRevert,
};
use casper_types::{ApiError, CLType, CLValue, ContractPackageHash, EntryPoint, EntryPointAccess, EntryPoints, EntryPointType, Key, Parameter, runtime_args, RuntimeArgs, U512, URef};

use casper_private_auction_core::{accounts, auction::Auction, bids::Bids, constructors, functions, keys};
use casper_private_auction_core::accounts::GIFT_ACCOUNT;
use casper_private_auction_core::data::AuctionData;
use casper_private_auction_core::error::AuctionError;
use casper_private_auction_core::gift::Gift;
use casper_private_auction_core::swap::Swap;
use casper_private_auction_core::utils::string_to_account_hash;

fn check_admin() {
    if string_to_account_hash(GIFT_ACCOUNT) != runtime::get_caller() {
        runtime::revert(AuctionError::InvalidCaller);
    }
}

#[no_mangle]
pub extern "C" fn claim() {
    // Only admin is allowed to call this
    check_admin();

    // All the details are passed in
    let receiver = runtime::get_named_arg::<Key>(keys::RECEIVER);
    let token_id = runtime::get_named_arg::<String>(keys::TOKEN_ID);

    Gift::claim(receiver, token_id);
}

#[no_mangle]
pub extern "C" fn cancel() {
    // Only admin is allowed to call this
    check_admin();

    // Get the arguments
    let token_id = runtime::get_named_arg::<String>(keys::TOKEN_ID);

    Gift::cancel(token_id);
}

#[no_mangle]
pub extern "C" fn gift()  {
    // Only admin is allowed to call this
    check_admin();

    // Get the arguments
    let sender = runtime::get_named_arg::<Key>(keys::SENDER);
    let token_id = runtime::get_named_arg::<String>(keys::TOKEN_ID);
    let token_package_hash = runtime::get_named_arg::<Key>(keys::TOKEN_PACKAGE_HASH)
        .into_hash()
        .unwrap_or_revert_with(AuctionError::MissingTokenPackageHash);

    Gift::gift(sender, token_id, ContractPackageHash::from(token_package_hash));
}

pub fn get_entry_points() -> EntryPoints {
    let mut entry_points = EntryPoints::new();

    entry_points.add_entry_point(EntryPoint::new(
        functions::CLAIM,
        vec![
            Parameter::new(keys::RECEIVER, CLType::Key),
            Parameter::new(keys::TOKEN_ID, CLType::String),
        ],
        CLType::Unit,
        EntryPointAccess::Public,
        EntryPointType::Contract,
    ));

    entry_points.add_entry_point(EntryPoint::new(
        functions::CANCEL,
        vec![
            Parameter::new(keys::SENDER, CLType::Key),
            Parameter::new(keys::TOKEN_ID, CLType::String),
        ],
        CLType::Unit,
        EntryPointAccess::Public,
        EntryPointType::Contract,
    ));

    entry_points.add_entry_point(EntryPoint::new(
        functions::GIFT,
        vec![
            Parameter::new(keys::SENDER, CLType::Key),
            Parameter::new(keys::TOKEN_ID, CLType::String),
            Parameter::new(keys::TOKEN_PACKAGE_HASH, CLType::Key),
        ],
        CLType::Unit,
        EntryPointAccess::Public,
        EntryPointType::Contract,
    ));

    entry_points
}

#[no_mangle]
pub extern "C" fn call() {
    let entry_points = get_entry_points();

    // let owner = runtime::get_named_arg::<Key>(keys::OWNER);
    // let token_id = runtime::get_named_arg::<String>(keys::TOKEN_ID);
    // let token_contract_hash = ContractPackageHash::from(
    //     runtime::get_named_arg::<Key>(keys::TOKEN_PACKAGE_HASH)
    //         .into_hash()
    //         .unwrap_or_revert_with(ApiError::User(200)),
    // );
    //
    let contract_name: String = runtime::get_named_arg("contract_name");
    let named_keys = constructors::create_gift_named_keys();

    let (contract_hash, _) = storage::new_contract(
        entry_points.into(),
        named_keys.into(),
        Some(String::from(&format!("{}_contract_package_hash", contract_name))),
        Some(String::from(&format!("{}_access_token", contract_name))),
    );

    let package_hash = ContractPackageHash::new(
        runtime::get_key(&format!("{}_contract_package_hash", contract_name))
            .unwrap_or_revert()
            .into_hash()
            .unwrap_or_revert(),
    );

    // Store contract in the account's named keys.
    runtime::put_key(
        &format!("{}_contract_hash", contract_name),
        contract_hash.into(),
    );
    runtime::put_key(
        &format!("{}_contract_hash_wrapped", contract_name),
        storage::new_uref(contract_hash).into(),
    );
    runtime::put_key(
        &format!("{}_package_hash_wrapped", contract_name),
        storage::new_uref(package_hash).into(),
    );
}
