use alloc::string::String;
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
        let (owner, package_hash) = gifts._get::<(Key, ContractPackageHash)>(token_id.as_str())
            .unwrap_or_revert_with(AuctionError::TokenNotFound);
        // Remove this key
        gifts._remove::<(Key, ContractPackageHash)>(token_id.as_str());

        let gift_contract = Gift::get_gift_contract();

        // Transfer the token back to the owner
        let token_ids = vec![token_id.clone()];
        runtime::call_versioned_contract::<()>(
            package_hash,
            None,
            "transfer_from",
            runtime_args! {
                "sender" => gift_contract,
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

        let gift_contract = Gift::get_gift_contract();

        // Transfer the token back to the owner
        let token_ids = vec![token_id];
        runtime::call_versioned_contract::<()>(
            package_hash,
            None,
            "transfer_from",
            runtime_args! {
                "sender" => gift_contract,
                "recipient" => Key::Account(owner.into_account().unwrap_or_revert_with(AuctionError::KeyNotAccount)),
                "token_ids" => token_ids,
            },
        );

        emit(&AuctionEvent::Cancelled {})
    }

    pub fn gift(sender: Key, token_id: String, token_package_hash: ContractPackageHash) {
        // Check the current token owner is the sender
        let current_owner = runtime::call_versioned_contract::<Option<Key>>(
            token_package_hash,
            None,
            "owner_of",
            runtime_args! {
                "token_id" => token_id.clone()
            },
        );
        if current_owner.is_none() {
            runtime::revert(AuctionError::InvalidCaller);
        }
        let owner = current_owner.unwrap();
        if owner != sender {
            runtime::revert(AuctionError::InvalidCaller);
        }
        // Create the mapping in the dictionary
        let gifts = crate::Dict::at(keys::TOKENS);
        gifts.set(token_id.as_str(), (sender.clone(), token_package_hash.clone()));

        let gift_contract = Gift::get_gift_contract();

        // Need to take ownership of the token
        let token_ids = vec![token_id.clone()];
        runtime::call_versioned_contract::<()>(
            token_package_hash,
            None,
            "transfer_from",
            runtime_args! {
                "sender" => Key::Account(owner.into_account().unwrap_or_revert_with(AuctionError::KeyNotAccount)),
                "recipient" => gift_contract,
                "token_ids" => token_ids,
            },
        );

        emit(&AuctionEvent::Gifted { account: sender.into_account().unwrap(), token_id })
    }

    fn get_gift_contract() -> Key {

        // let contract_name = read_named_key_value::<String>(keys::NAME);
        // let package_hash = ContractPackageHash::new(
        //     runtime::get_key(&format!("{}_contract_package_hash", contract_name))
        //         .unwrap_or_revert_with(AuctionError::ContractPackageNotFound)
        //         .into_hash()
        //         .unwrap_or_revert_with(AuctionError::ContractPackageNotFound)
        // );

        {
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
        }
    }
}


