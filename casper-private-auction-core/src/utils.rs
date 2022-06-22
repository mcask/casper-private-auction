use alloc::format;
use alloc::string::ToString;
use casper_contract::contract_api::runtime;
use casper_contract::contract_api::runtime::revert;
use casper_contract::contract_api::storage::new_dictionary;
use casper_contract::unwrap_or_revert::UnwrapOrRevert;
use casper_types::account::AccountHash;
use casper_types::contracts::NamedKeys;
use crate::AuctionError;

pub fn add_empty_dict(named_keys: &mut NamedKeys, name: &str) {
    if runtime::get_key(name).is_some() {
        runtime::remove_key(name);
    }
    let dict = new_dictionary(name).unwrap_or_revert_with(AuctionError::CannotCreateDictionary);
    named_keys.insert(name.to_string(), dict.into());
}

pub fn string_to_account_hash(account_string: &str) -> AccountHash {
    let account = if account_string.starts_with("account-hash-") {
        AccountHash::from_formatted_str(account_string)
    } else if account_string.starts_with("Key::Account(") {
        AccountHash::from_formatted_str(
            account_string
                .replace("Key::Account(", "account-hash-")
                .strip_suffix(')')
                .unwrap_or_revert(),
        )
    } else {
        AccountHash::from_formatted_str(&format!("account-hash-{}", account_string))
    };
    match account {
        Ok(acc) => acc,
        Err(_e) => revert(AuctionError::CommissionAccountIncorrectSerialization),
    }
}

pub fn string_to_u16(ustr: &str) -> u16 {
    match ustr.parse::<u16>() {
        Ok(u) => u,
        Err(_e) => revert(AuctionError::CommissionRateIncorrectSerialization),
    }
}