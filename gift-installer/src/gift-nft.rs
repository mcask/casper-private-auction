#![no_std]
#![no_main]
extern crate alloc;
use alloc::string::String;
use alloc::vec;

use casper_contract::{
    contract_api::{
        runtime::{self, revert},
    },
    unwrap_or_revert::UnwrapOrRevert,
};
use casper_types::{runtime_args, RuntimeArgs, Key, ContractPackageHash};
use casper_private_auction_core::{keys};
use casper_private_auction_core::error::AuctionError;

#[no_mangle]
pub extern "C" fn call() {
    let gift_contract = ContractPackageHash::from(
        runtime::get_named_arg::<Key>("gift_contract")
            .into_hash()
            .unwrap_or_revert_with(AuctionError::ContractPackageNotFound)
    );
    let sender = runtime::get_named_arg::<Key>(keys::SENDER);
    let token_id = runtime::get_named_arg::<String>(keys::TOKEN_ID);
    let token_package_hash = runtime::get_named_arg::<Key>(keys::TOKEN_PACKAGE_HASH);

    let tp = ContractPackageHash::from(
        token_package_hash
            .into_hash()
            .unwrap_or_revert_with(AuctionError::ContractPackageNotFound)
    );

    // Check the current token owner is the sender
    let current_owner = runtime::call_versioned_contract::<Option<Key>>(
        tp,
        None,
        "owner_of",
        runtime_args! {
                "token_id" => token_id.clone()
            }
    );
    if current_owner.is_none() || current_owner.unwrap() != sender {
        revert(AuctionError::InvalidCaller);
    }

    runtime::call_versioned_contract::<()>(
        gift_contract,
        None,
        "gift",
        runtime_args! {
            "sender" => sender,
            "token_id" => token_id.clone(),
            "token_package_hash" => token_package_hash
        }
    );

    // Transfer the ownership of the token to the contract
    let token_ids = vec![token_id];
    runtime::call_versioned_contract::<()>(
        tp,
        None,
        "transfer_from",
        runtime_args! {
                "sender" => Key::Account(sender.into_account().unwrap_or_revert_with(AuctionError::KeyNotAccount)),
                "recipient" => Key::Hash(gift_contract.value()),
                "token_ids" => token_ids,
            }
    );

}
