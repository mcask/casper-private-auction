use alloc::string::{String};
use alloc::{vec};
use casper_contract::contract_api::runtime;
use casper_contract::unwrap_or_revert::UnwrapOrRevert;
pub use casper_types::{
    ApiError, bytesrepr::FromBytes, CLTyped, ContractHash, contracts::NamedKeys,
    Key, runtime_args, RuntimeArgs, system::CallStackElement, U512, URef,
};
pub use casper_types::bytesrepr::ToBytes;
use casper_types::ContractPackageHash;

use crate::{events::{AuctionEvent, emit}, keys};
use crate::error::AuctionError;

pub struct Gift;

impl Gift {

    pub fn claim(receiver: Key, token_id: String) {
        let gifts = crate::Dict::at(keys::TOKENS);
        // Get the token
        let (_, package_hash) = gifts._get::<(Key, ContractPackageHash)>(token_id.as_str())
            .unwrap_or_revert_with(AuctionError::TokenNotFound);
        // Remove this key
        gifts._remove::<(Key, ContractPackageHash)>(token_id.as_str());

        // Transfer the token back to the owner
        let token_ids = vec![token_id.clone()];
        runtime::call_versioned_contract::<()>(
            package_hash,
            None,
            "transfer",
            runtime_args! {
                "recipient" => Key::Account(receiver.into_account().unwrap_or_revert_with(AuctionError::KeyNotAccount)),
                "token_ids" => token_ids,
            },
        );

        emit(&AuctionEvent::Claimed { account: receiver.into_account().unwrap(), token_id })
    }

    pub fn cancel(token_id: String) {
        let gifts = crate::Dict::at(keys::TOKENS);
        // Get the token
        let (owner, package_hash) = gifts._get::<(Key, ContractPackageHash)>(token_id.as_str())
            .unwrap_or_revert_with(AuctionError::TokenNotFound);
        // Remove this key
        gifts._remove::<(Key, ContractPackageHash)>(token_id.as_str());

        // Transfer the token back to the owner
        let token_ids = vec![token_id];
        runtime::call_versioned_contract::<()>(
            package_hash,
            None,
            "transfer",
            runtime_args! {
                "recipient" => Key::Account(owner.into_account().unwrap_or_revert_with(AuctionError::KeyNotAccount)),
                "token_ids" => token_ids,
            },
        );

        emit(&AuctionEvent::Cancelled {})
    }

    pub fn gift(sender: Key, token_id: String, token_package_hash: ContractPackageHash) {
        // Create the mapping in the dictionary
        let gifts = crate::Dict::at(keys::TOKENS);
        gifts.set(token_id.as_str(), (sender.clone(), token_package_hash.clone()));

        emit(&AuctionEvent::Gifted { account: sender.into_account().unwrap(), token_id })
    }
    //
    // fn get_gift_contract() -> Key {
    //     {
    //         let call_stack = runtime::get_call_stack();
    //         let caller: CallStackElement = call_stack
    //             .last()
    //             .unwrap_or_revert_with(AuctionError::CallStackTooShort)
    //             .clone();
    //         match caller {
    //             CallStackElement::StoredContract {
    //                 contract_package_hash,
    //                 contract_hash: _,
    //             } => Key::Hash(contract_package_hash.value()),
    //             _ => runtime::revert(AuctionError::InvalidCaller),
    //         }
    //     }
    // }
}


