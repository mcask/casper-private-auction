#![no_std]

extern crate alloc;

use casper_contract::{
    contract_api::{runtime, storage},
    unwrap_or_revert::UnwrapOrRevert,
};
use casper_types::{
    bytesrepr::{FromBytes, ToBytes},
    CLTyped, Key, URef,
};

use error::AuctionError;


pub mod auction;
pub mod error;
#[macro_use]
pub mod data;
pub mod bids;
pub mod events;
pub mod keys;
pub mod functions;
pub mod accounts;
pub mod utils;
pub mod constructors;
pub mod english;
pub mod dutch;
pub mod swap;
pub mod gift;

struct Dict {
    uref: URef,
}

impl Dict {
    pub fn at(name: &str) -> Dict {
        let key: Key =
            runtime::get_key(name).unwrap_or_revert_with(AuctionError::DictionaryKeyNotFound);
        let uref: URef = *key
            .as_uref()
            .unwrap_or_revert_with(AuctionError::DictionaryKeyNotURef);
        Dict { uref }
    }

    pub fn _get<T: CLTyped + FromBytes>(&self, key: &str) -> Option<T> {
        storage::dictionary_get(self.uref, key)
            .unwrap_or_revert_with(AuctionError::DictionaryGetFail)
            .unwrap_or_default()
    }

    pub fn set<T: CLTyped + ToBytes>(&self, key: &str, value: T) {
        storage::dictionary_put(self.uref, key, Some(value));
    }

    pub fn _remove<T: CLTyped + ToBytes>(&self, key: &str) {
        storage::dictionary_put(self.uref, key, Option::<T>::None);
    }
}
