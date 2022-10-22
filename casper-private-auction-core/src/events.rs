use crate::error::AuctionError;
use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use casper_contract::contract_api::runtime::{self, revert};
use casper_contract::contract_api::storage;
use casper_contract::unwrap_or_revert::UnwrapOrRevert;
use casper_types::Key;
use casper_types::{account::AccountHash, U512};
use crate::keys::{EVENTS, EVENTS_COUNT};

pub enum AuctionEvent {
    Bid {
        account: AccountHash,
        bid: U512,
        synthetic: bool,
    },
    BidCancelled {
        account: AccountHash,
    },
    Cancelled {
    },
    PendingSettlement {
        account: AccountHash,
        bid: (U512, bool),
    },
    SettlementRejected {
        account: Option<AccountHash>,
    },
    Settled {
        account: Option<AccountHash>,
        bid: Option<(U512, bool)>,
    },
    Gifted {
        account: AccountHash,
        token_id: String,
    },
    Claimed {
        account: AccountHash,
        token_id: String,
    },
}

pub fn emit(event: &AuctionEvent) {
    let mut events_count = get_events_count();

    let (emit_event, event_id): (BTreeMap<&str, String>, String) = match event {
        AuctionEvent::Bid { account, bid, synthetic } => {
            let mut event = BTreeMap::new();
            let event_id = events_count.to_string();
            event.insert("event_id", event_id.clone());
            event.insert("account", account.to_string());
            event.insert("event_type", "Bid".to_string());
            event.insert("bid", bid.to_string());
            event.insert("synthetic", synthetic.to_string());
            (event, event_id)
        }
        AuctionEvent::BidCancelled { account } => {
            let mut event = BTreeMap::new();
            let event_id = events_count.to_string();
            event.insert("event_id", event_id.clone());
            event.insert("account", account.to_string());
            event.insert("event_type", "BidCancelled".to_string());
            (event, event_id)
        }
        AuctionEvent::Cancelled { } => {
            let mut event = BTreeMap::new();
            let event_id = events_count.to_string();
            event.insert("event_id", event_id.clone());
            event.insert("event_type", "Cancelled".to_string());
            (event, event_id)
        }
        AuctionEvent::PendingSettlement { account, bid } => {
            let mut event = BTreeMap::new();
            let event_id = events_count.to_string();
            event.insert("event_id", event_id.clone());
            event.insert("account", account.to_string());
            event.insert("bid", bid.0.to_string());
            event.insert("synthetic", bid.1.to_string());
            event.insert("event_type", "PendingSettlement".to_string());
            (event, event_id)
        }
        AuctionEvent::SettlementRejected { account } => {
            let mut event = BTreeMap::new();
            let event_id = events_count.to_string();
            event.insert("event_id", event_id.clone());
            if account.is_some() {
                event.insert("account", account.unwrap().to_string());
            }
            event.insert("event_type", "SettlementRejected".to_string());
            (event, event_id)
        }
        AuctionEvent::Settled { account, bid } => {
            let mut event = BTreeMap::new();
            let event_id = events_count.to_string();
            event.insert("event_id", event_id.clone());
            if account.is_some() {
                event.insert("account", account.unwrap().to_string());
            }
            if bid.is_some() {
                let wb = bid.unwrap();
                event.insert("bid", wb.0.to_string());
                event.insert("synthetic", wb.1.to_string());
            }
            event.insert("event_type", "Settled".to_string());
            (event, event_id)
        }
        AuctionEvent::Gifted { account, token_id } => {
            let mut event = BTreeMap::new();
            let event_id = events_count.to_string();
            event.insert("event_id", event_id.clone());
            event.insert("account", account.to_string());
            event.insert("token_id", token_id.clone());
            event.insert("event_type", "Gifted".to_string());
            (event, event_id)
        }
        AuctionEvent::Claimed { account, token_id } => {
            let mut event = BTreeMap::new();
            let event_id = events_count.to_string();
            event.insert("event_id", event_id.clone());
            event.insert("account", account.to_string());
            event.insert("token_id", token_id.clone());
            event.insert("event_type", "Claimed".to_string());
            (event, event_id)
        }
    };
    events_count += 1;

    let events_dict = crate::Dict::at(EVENTS);
    events_dict.set(&event_id, emit_event);
    set_events_count(events_count);
}

pub fn get_events_count() -> u32 {
    if let Some(Key::URef(uref)) = runtime::get_key(EVENTS_COUNT) {
        return storage::read(uref)
            .unwrap_or_revert_with(AuctionError::CannotReadKey)
            .unwrap_or_revert_with(AuctionError::NamedKeyNotFound);
    }
    revert(AuctionError::BadKey)
}

pub fn set_events_count(events_count: u32) {
    match runtime::get_key(EVENTS_COUNT) {
        Some(key) => {
            if let Key::URef(uref) = key {
                storage::write(uref, events_count);
            }
        }
        None => {
            let key = storage::new_uref(events_count).into();
            runtime::put_key(EVENTS_COUNT, key);
        }
    }
}
