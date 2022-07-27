use casper_types::U512;
use casper_private_auction_core::accounts::MARKETPLACE_COMMISSION;
use crate::swap_args::AuctionArgBuilder;
use crate::swap_auction::SwapAuctionContract;
use crate::utils;
use crate::auction::BaseAuctionArgs;

#[test]
fn deploy_auction() {
    let now = utils::get_now_u64();
    SwapAuctionContract::deploy_with_default_args(now);
}

#[test]
#[should_panic = "User(9)"]
fn deploy_auction_invalid_times() {
    let now = utils::get_now_u64();
    let mut auction_args = AuctionArgBuilder::default();
    auction_args.set_start_time(now);
    auction_args.set_end_time(now - 1000);
    SwapAuctionContract::deploy(auction_args);
}

#[test]
fn cancel_auction() {
    let now = utils::get_now_u64();
    let mut auction = SwapAuctionContract::deploy_with_default_args(now);
    let (admin, _, _, _, _, _) = auction.contract.accounts;

    auction.cancel_auction(&admin, now + 1001)
}

#[test]
#[should_panic = "User(1)"]
fn cancel_auction_not_owner() {
    let now = utils::get_now_u64();
    let mut auction = SwapAuctionContract::deploy_with_default_args(now);
    let (_, _, _, _, bob, _) = auction.contract.accounts;

    auction.cancel_auction(&bob, now + 1001)
}

#[test]
#[should_panic = "User(11)"]
fn early_bid() {
    let now = utils::get_now_u64();
    let auction_args = AuctionArgBuilder::base(
        now + 1000,
        U512::from(10000),
        100
    );
    let mut auction = SwapAuctionContract::deploy(auction_args);
    let (_, _, _, _, bob, _) = auction.contract.accounts;

    auction.bid(&bob, U512::from(1200), now);
}

#[test]
#[should_panic = "User(2)"]
fn late_bid() {
    let now = utils::get_now_u64();
    let auction_args = AuctionArgBuilder::base(
        now,
        U512::from(10000),
        100
    );
    let mut auction = SwapAuctionContract::deploy(auction_args);
    let (_, _, _, _, bob, _) = auction.contract.accounts;

    auction.bid(&bob, U512::from(12000), now + 10000);
}

#[test]
#[should_panic = "User(10)"]
fn low_bid() {
    let now = utils::get_now_u64();
    let auction_args = AuctionArgBuilder::base(
        now,
        U512::from(10000),
        100
    );
    let mut auction = SwapAuctionContract::deploy(auction_args);
    let (_, _, _, _, bob, _) = auction.contract.accounts;

    // This fails because the wrong amount is transferred from the purse
    auction.bid(&bob, U512::from(8000), now + 1000);
}

#[test]
#[should_panic = "User(18)"]
fn no_kyc_bid() {
    let now = utils::get_now_u64();
    let auction_args = AuctionArgBuilder::base(
        now,
        U512::from(10000),
        100
    );
    let mut auction = SwapAuctionContract::deploy(auction_args);
    let (_, _, _, _, _, dan) = auction.contract.accounts;

    // This fails because the wrong amount is transferred from the purse
    auction.bid(&dan, U512::from(10000), now + 1000);
}

#[test]
fn bid() {
    let now = utils::get_now_u64();
    let auction_args = AuctionArgBuilder::base(
        now,
        U512::from(10000),
        100_u32
    );
    let mut auction = SwapAuctionContract::deploy(auction_args);
    let (_, _, _, _, bob, _) = auction.contract.accounts;

    let bid_price = U512::from(10000);
    // Compute marketplace commission
    let mkt_com = (bid_price.as_u32() / 1000) * MARKETPLACE_COMMISSION;

    // Now hit the price
    auction.bid(&bob, bid_price, now + 1000);
    // This should finalize the auction
    {
        let (winner, bid) = auction.contract.get_current_winner();
        assert!(winner.is_some());
        assert_eq!(winner.unwrap(), bob);
        assert!(bid.is_some());
        assert_eq!(bid.unwrap().0, bid_price);
        assert!(!bid.unwrap().1);
        assert!(!auction.contract.is_live());
        assert!(auction.contract.is_settled());
        let (_, mb, arb, _, _, _) = auction.contract.get_balances();
        assert_eq!(mb, U512::from(mkt_com + 1)); // starting funds are 1
        assert_eq!(arb, U512::from(976_u64));
    }
}

#[test]
#[should_panic = "User(58)"]
fn synth_bid_no_permission() {
    let now = utils::get_now_u64();
    let auction_args = AuctionArgBuilder::base(
        now,
        U512::from(10000),
        100_u32
    );
    let mut auction = SwapAuctionContract::deploy(auction_args);
    let (_, market, _, _, bob, _) = auction.contract.accounts;
    auction.contract.transfer_funds(&market, U512::from(100_000_000_000_000_u64));
    // Now hit the price
    auction.synthetic_bid(&market, &bob, U512::from(10000), now + 1000);
}

#[test]
#[should_panic = "User(10)"]
fn synth_bid_too_low() {
    let now = utils::get_now_u64();
    let auction_args = AuctionArgBuilder::base(
        now,
        U512::from(10000),
        100_u32
    );
    let mut auction = SwapAuctionContract::deploy(auction_args);
    let (_, market, _, ali, _, _) = auction.contract.accounts;
    auction.contract.transfer_funds(&market, U512::from(100_000_000_000_000_u64));
    // Now hit the price
    auction.synthetic_bid(&market, &ali, U512::from(8000), now + 1000);
}

#[test]
fn synth_bid() {
    let now = utils::get_now_u64();
    let bid_price = U512::from(10000_u64);
    let auction_args = AuctionArgBuilder::base(
        now,
        bid_price.clone(),
        100_u32
    );
    let mut auction = SwapAuctionContract::deploy(auction_args);
    let (_, market, _, ali, _, _) = auction.contract.accounts;
    auction.contract.transfer_funds(&market, U512::from(100_000_000_000_000_u64));
    // Now hit the price
    auction.synthetic_bid(&market, &ali, bid_price.clone(), now + 1000);
    // This should put auction to pending settlement
    {
        let (winner, bid) = auction.contract.get_current_winner();
        assert!(winner.is_some());
        assert_eq!(winner.unwrap(), ali);
        assert!(bid.is_some());
        assert_eq!(bid.unwrap().0, bid_price);
        assert!(bid.unwrap().1);
        assert!(!auction.contract.is_live());
        assert!(auction.contract.is_pending_settle());
    }
}

#[test]
#[should_panic = "User(1)"]
fn synth_bid_approve_no_permission() {
    let now = utils::get_now_u64();
    let bid_price = U512::from(10000_u64);
    let auction_args = AuctionArgBuilder::base(
        now,
        bid_price.clone(),
        100_u32
    );
    let mut auction = SwapAuctionContract::deploy(auction_args);
    let (_, market, _, ali, _, _) = auction.contract.accounts;
    auction.contract.transfer_funds(&market, U512::from(100_000_000_000_000_u64));
    // Now hit the price
    auction.synthetic_bid(&market, &ali, bid_price.clone(), now + 1000);
    // This should put auction to pending settlement
    {
        let (winner, bid) = auction.contract.get_current_winner();
        assert!(winner.is_some());
        assert_eq!(winner.unwrap(), ali);
        assert!(bid.is_some());
        assert_eq!(bid.unwrap().0, bid_price);
        assert!(bid.unwrap().1);
        assert!(!auction.contract.is_live());
        assert!(auction.contract.is_pending_settle());
    }
    auction.approve(&ali, now + 2000);
}

#[test]
fn synth_bid_approve() {
    let now = utils::get_now_u64();
    let bid_price = U512::from(10000_u64);
    let auction_args = AuctionArgBuilder::base(
        now,
        bid_price.clone(),
        100_u32
    );
    let mut auction = SwapAuctionContract::deploy(auction_args);
    let (_, market, _, ali, _, _) = auction.contract.accounts;
    auction.contract.transfer_funds(&market, U512::from(100_000_000_000_000_u64));
    // Now hit the price
    auction.synthetic_bid(&market, &ali, bid_price.clone(), now + 1000);
    // This should put auction to pending settlement
    {
        let (winner, bid) = auction.contract.get_current_winner();
        assert!(winner.is_some());
        assert_eq!(winner.unwrap(), ali);
        assert!(bid.is_some());
        assert_eq!(bid.unwrap().0, bid_price);
        assert!(bid.unwrap().1);
        assert!(!auction.contract.is_live());
        assert!(auction.contract.is_pending_settle());
    }
    auction.approve(&market, now + 2000);
    {
        assert!(!auction.contract.is_live());
        assert!(auction.contract.is_settled());
        let (_, _, arb, _, _, _) = auction.contract.get_balances();
        // There should be no fund movement
        assert_eq!(arb, U512::from(1_u64));
    }
}

#[test]
#[should_panic = "User(1)"]
fn synth_bid_reject_no_permission() {
    let now = utils::get_now_u64();
    let bid_price = U512::from(30000_u64);
    let auction_args = AuctionArgBuilder::base(
        now,
        bid_price.clone(),
        100_u32
    );
    let mut auction = SwapAuctionContract::deploy(auction_args);
    let (_, market, _, ali, _, _) = auction.contract.accounts;
    auction.contract.transfer_funds(&market, U512::from(100_000_000_000_000_u64));
    // Now hit the price
    auction.synthetic_bid(&market, &ali, bid_price.clone(), now + 1000);
    // This should put auction to pending settlement
    {
        let (winner, bid) = auction.contract.get_current_winner();
        assert!(winner.is_some());
        assert_eq!(winner.unwrap(), ali);
        assert!(bid.is_some());
        assert_eq!(bid.unwrap().0, bid_price);
        assert!(bid.unwrap().1);
        assert!(!auction.contract.is_live());
        assert!(auction.contract.is_pending_settle());
    }
    auction.reject(&ali, now + 2000);
}

#[test]
fn synth_bid_reject() {
    let now = utils::get_now_u64();
    let bid_price = U512::from(10000_u64);
    let auction_args = AuctionArgBuilder::base(
        now,
        bid_price.clone(),
        100_u32
    );
    let mut auction = SwapAuctionContract::deploy(auction_args);
    let (_, market, _, ali, _, _) = auction.contract.accounts;
    auction.contract.transfer_funds(&market, U512::from(100_000_000_000_000_u64));
    // Now hit the price
    auction.synthetic_bid(&market, &ali, bid_price.clone(), now + 1000);
    // This should put auction to pending settlement
    {
        let (winner, bid) = auction.contract.get_current_winner();
        assert!(winner.is_some());
        assert_eq!(winner.unwrap(), ali);
        assert!(bid.is_some());
        assert_eq!(bid.unwrap().0, bid_price);
        assert!(bid.unwrap().1);
        assert!(!auction.contract.is_live());
        assert!(auction.contract.is_pending_settle());
    }
    auction.reject(&market, now + 2000);
    {
        assert!(!auction.contract.is_live());
        assert!(auction.contract.is_rejected());
        let (_, _, arb, _, _, _) = auction.contract.get_balances();
        // There should be no fund movement
        assert_eq!(arb, U512::from(1_u64));
    }
}

#[test]
#[should_panic = "User(4)"]
fn bid_after_settle() {
    let now = utils::get_now_u64();
    let auction_args = AuctionArgBuilder::base(
        now,
        U512::from(10000),
        100_u32
    );
    let mut auction = SwapAuctionContract::deploy(auction_args);
    let (_, _, _, _, bob, _) = auction.contract.accounts;

    let bid_price = U512::from(10000);
    // Compute marketplace commission
    let mkt_com = (bid_price.as_u32() / 1000) * MARKETPLACE_COMMISSION;

    // Now hit the price
    auction.bid(&bob, bid_price, now + 1000);
    // This should finalize the auction
    {
        let (winner, bid) = auction.contract.get_current_winner();
        assert!(winner.is_some());
        assert_eq!(winner.unwrap(), bob);
        assert!(bid.is_some());
        assert_eq!(bid.unwrap().0, bid_price);
        assert!(!bid.unwrap().1);
        assert!(!auction.contract.is_live());
        assert!(auction.contract.is_settled());
        let (_, mb, arb, _, _, _) = auction.contract.get_balances();
        assert_eq!(mb, U512::from(mkt_com + 1)); // starting funds are 1
        assert_eq!(arb, U512::from(976_u64));
    }
    auction.bid(&bob, bid_price, now + 2000);
}

#[test]
#[should_panic = "User(4)"]
fn synth_bid_after_settle() {
    let now = utils::get_now_u64();
    let bid_price = U512::from(10000_u64);
    let auction_args = AuctionArgBuilder::base(
        now,
        bid_price.clone(),
        100_u32
    );
    let mut auction = SwapAuctionContract::deploy(auction_args);
    let (_, market, _, ali, _, _) = auction.contract.accounts;
    auction.contract.transfer_funds(&market, U512::from(100_000_000_000_000_u64));
    // Now hit the price
    auction.synthetic_bid(&market, &ali, bid_price.clone(), now + 1000);
    // This should put auction to pending settlement
    {
        let (winner, bid) = auction.contract.get_current_winner();
        assert!(winner.is_some());
        assert_eq!(winner.unwrap(), ali);
        assert!(bid.is_some());
        assert_eq!(bid.unwrap().0, bid_price);
        assert!(bid.unwrap().1);
        assert!(!auction.contract.is_live());
        assert!(auction.contract.is_pending_settle());
    }
    auction.synthetic_bid(&market, &ali, bid_price.clone(), now + 2000);
}

// // Fails with BadState (User(5)) error since on bidding the contract notices that it was already finalized.
// // User(5) might also be either that the auction managed to be finalized before expiring, or Dutch contract was initialized without starting price.
// #[test]
// #[should_panic = "User(5)"]
// fn dutch_auction_already_taken_test() {
//     let now = auction_args::AuctionArgsBuilder::get_now_u64();
//     let mut auction_args = auction_args::AuctionArgsBuilder::default();
//     auction_args.set_starting_price(Some(U512::from(40000)));
//     auction_args.set_dutch();
//     let mut auction = auction::AuctionContract::deploy_contracts(auction_args);
//     auction.bid(&auction.bob.clone(), U512::from(40000), now + 1000);
//     auction.bid(&auction.bob.clone(), U512::from(40000), now + 1001);
// }
//
// // User(6) error -> trying to cancel a bid that wasn't placed
// #[test]
// #[should_panic = "User(6)"]
// fn english_auction_no_bid_cancel_test() {
//     let now = auction_args::AuctionArgsBuilder::get_now_u64();
//     let mut auction = auction::AuctionContract::deploy_with_default_args(true, now);
//     auction.cancel_bid(&auction.bob.clone(), now + 2000);
// }
//
// #[test]
// #[should_panic = "User(7)"]
// fn english_auction_bid_late_cancel_test() {
//     let now = auction_args::AuctionArgsBuilder::get_now_u64();
//     let mut auction = auction::AuctionContract::deploy_with_default_args(true, now);
//     auction.bid(&auction.bob.clone(), U512::from(40000), now + 1);
//     auction.cancel_bid(&auction.bob.clone(), now + 3000);
// }
//
// // Deploying an auction with neither ENGLISH nor DUTCH format results in User(8) error
// #[test]
// #[should_panic = "User(8)"]
// fn auction_unknown_format_test() {
//     let admin_secret = SecretKey::ed25519_from_bytes([1u8; 32]).unwrap();
//     let ali_secret = SecretKey::ed25519_from_bytes([3u8; 32]).unwrap();
//     let bob_secret = SecretKey::ed25519_from_bytes([5u8; 32]).unwrap();
//
//     let admin_pk: PublicKey = PublicKey::from(&admin_secret);
//     let admin = admin_pk.to_account_hash();
//     let ali_pk: PublicKey = PublicKey::from(&ali_secret);
//     let ali = ali_pk.to_account_hash();
//     let bob_pk: PublicKey = PublicKey::from(&bob_secret);
//     let bob = bob_pk.to_account_hash();
//
//     let mut builder = InMemoryWasmTestBuilder::default();
//     builder.run_genesis(&DEFAULT_RUN_GENESIS_REQUEST).commit();
//     builder.exec(fund_account(&admin)).expect_success().commit();
//     builder.exec(fund_account(&ali)).expect_success().commit();
//     builder.exec(fund_account(&bob)).expect_success().commit();
//
//     let (kyc_hash, kyc_package) = AuctionContract::deploy_kyc(&mut builder, &admin);
//
//     AuctionContract::add_kyc(&mut builder, &kyc_package, &admin, &admin);
//     AuctionContract::add_kyc(&mut builder, &kyc_package, &admin, &ali);
//     AuctionContract::add_kyc(&mut builder, &kyc_package, &admin, &bob);
//
//     let (nft_hash, nft_package) = AuctionContract::deploy_nft(&mut builder, &admin, kyc_package);
//     let token_id = String::from("custom_token_id");
//
//     let auction_args = runtime_args! {
//         "beneficiary_account"=>Key::Account(admin),
//         "token_contract_hash"=>Key::Hash(nft_package.value()),
//         "kyc_package_hash" => Key::Hash(kyc_package.value()),
//         "format"=> "WOLOLO",
//         "starting_price"=> None::<U512>,
//         "reserve_price"=>U512::from(300),
//         "token_id"=>token_id,
//         "start_time" => 1,
//         "cancellation_time" => 2,
//         "end_time" => 3,
//         "name" => "test",
//         "bidder_count_cap" => Some(10_u64),
//         "auction_timer_extension" => None::<u64>,
//         "minimum_bid_step"=> None::<U512>,
//         "marketplace_account" => AccountHash::new([11_u8; 32]),
//         "marketplace_commission" => 75
//     };
//
//     let (auction_hash, auction_package) =
//         AuctionContract::deploy_auction(&mut builder, &admin, auction_args);
// }
//
// // Deploying with wrong times reverts with User(9) error
// #[test]
// #[should_panic = "User(9)"]
// fn auction_bad_times_test() {
//     let admin_secret = SecretKey::ed25519_from_bytes([1u8; 32]).unwrap();
//     let ali_secret = SecretKey::ed25519_from_bytes([3u8; 32]).unwrap();
//     let bob_secret = SecretKey::ed25519_from_bytes([5u8; 32]).unwrap();
//
//     let admin_pk: PublicKey = PublicKey::from(&admin_secret);
//     let admin = admin_pk.to_account_hash();
//     let ali_pk: PublicKey = PublicKey::from(&ali_secret);
//     let ali = ali_pk.to_account_hash();
//     let bob_pk: PublicKey = PublicKey::from(&bob_secret);
//     let bob = bob_pk.to_account_hash();
//
//     let mut builder = InMemoryWasmTestBuilder::default();
//     builder.run_genesis(&DEFAULT_RUN_GENESIS_REQUEST).commit();
//     builder.exec(fund_account(&admin)).expect_success().commit();
//     builder.exec(fund_account(&ali)).expect_success().commit();
//     builder.exec(fund_account(&bob)).expect_success().commit();
//
//     let (kyc_hash, kyc_package) = AuctionContract::deploy_kyc(&mut builder, &admin);
//
//     AuctionContract::add_kyc(&mut builder, &kyc_package, &admin, &admin);
//     AuctionContract::add_kyc(&mut builder, &kyc_package, &admin, &ali);
//     AuctionContract::add_kyc(&mut builder, &kyc_package, &admin, &bob);
//
//     let (nft_hash, nft_package) = AuctionContract::deploy_nft(&mut builder, &admin, kyc_package);
//     let token_id = String::from("custom_token_id");
//
//     let auction_args = runtime_args! {
//         "beneficiary_account"=>Key::Account(admin),
//         "token_contract_hash"=>Key::Hash(nft_package.value()),
//         "kyc_package_hash" => Key::Hash(kyc_package.value()),
//         "format"=> "ENGLISH",
//         "starting_price"=> None::<U512>,
//         "reserve_price"=>U512::from(300),
//         "token_id"=>token_id,
//         "start_time" => 1000_u64,
//         "cancellation_time" => 20_u64,
//         "end_time" => 11_u64,
//         "name" => "test",
//         "bidder_count_cap" => Some(10_u64),
//         "auction_timer_extension" => None::<u64>,
//         "minimum_bid_step"=> None::<U512>,
//         "marketplace_account" => AccountHash::new([11_u8; 32]),
//         "marketplace_commission" => 75
//     };
//
//     let (auction_hash, auction_package) =
//         AuctionContract::deploy_auction(&mut builder, &admin, auction_args);
// }
//
// // Any combination of bad prices on auction deployment returns User(10)
// #[test]
// #[should_panic = "User(10)"]
// fn dutch_auction_no_starting_price_test() {
//     let now = auction_args::AuctionArgsBuilder::get_now_u64();
//     let mut auction_args = auction_args::AuctionArgsBuilder::default();
//     auction_args.set_starting_price(None);
//     auction_args.set_dutch();
//     let mut auction = auction::AuctionContract::deploy_contracts(auction_args);
//     auction.bid(&auction.bob.clone(), U512::from(40000), now + 1000);
// }
//
// #[test]
// #[should_panic = "User(11)"]
// fn english_auction_bid_early_test() {
//     let now = auction_args::AuctionArgsBuilder::get_now_u64();
//     let mut auction = auction::AuctionContract::deploy_with_default_args(true, now);
//     auction.bid(&auction.bob.clone(), U512::from(40000), now - 1000);
// }
//
// #[test]
// #[should_panic = "User(4)"]
// fn auction_bid_no_kyc_token_test() {
//     let admin_secret = SecretKey::ed25519_from_bytes([1u8; 32]).unwrap();
//     let ali_secret = SecretKey::ed25519_from_bytes([3u8; 32]).unwrap();
//     let bob_secret = SecretKey::ed25519_from_bytes([5u8; 32]).unwrap();
//
//     let admin_pk: PublicKey = PublicKey::from(&admin_secret);
//     let admin = admin_pk.to_account_hash();
//     let ali_pk: PublicKey = PublicKey::from(&ali_secret);
//     let ali = ali_pk.to_account_hash();
//     let bob_pk: PublicKey = PublicKey::from(&bob_secret);
//     let bob = bob_pk.to_account_hash();
//
//     let mut builder = InMemoryWasmTestBuilder::default();
//     builder.run_genesis(&DEFAULT_RUN_GENESIS_REQUEST).commit();
//     builder.exec(fund_account(&admin)).expect_success().commit();
//     builder.exec(fund_account(&ali)).expect_success().commit();
//     builder.exec(fund_account(&bob)).expect_success().commit();
//
//     let (kyc_hash, kyc_package) = AuctionContract::deploy_kyc(&mut builder, &admin);
//
//     let (nft_hash, nft_package) = AuctionContract::deploy_nft(&mut builder, &admin, kyc_package);
//     let token_id = String::from("custom_token_id");
//
//     let now: u64 = AuctionArgsBuilder::get_now_u64();
//     let auction_args = runtime_args! {
//         "beneficiary_account"=>Key::Account(admin),
//         "token_contract_hash"=>Key::Hash(nft_package.value()),
//         "kyc_package_hash" => Key::Hash(kyc_package.value()),
//         "format"=> "ENGLISH",
//         "starting_price"=> None::<U512>,
//         "reserve_price"=>U512::from(300),
//         "token_id"=>token_id,
//         "start_time" => now+500,
//         "cancellation_time" => now+3500,
//         "end_time" => now+4000,
//         "name" => "test",
//         "bidder_count_cap" => Some(10_u64),
//         "auction_timer_extension" => None::<u64>,
//         "minimum_bid_step"=> None::<U512>,
//         "marketplace_account" => AccountHash::new([11_u8; 32]),
//         "marketplace_commission" => 75
//     };
//
//     let (auction_hash, auction_package) =
//         AuctionContract::deploy_auction(&mut builder, &admin, auction_args);
//     //bid
//     let session_code = PathBuf::from("bid-purse.wasm");
//     deploy(
//         &mut builder,
//         &admin,
//         &DeploySource::Code(session_code),
//         runtime_args! {
//             "bid" => U512::from(40000),
//             "auction" => auction_hash
//         },
//         true,
//         Some(now + 1500),
//     );
// }
//
// #[test]
// fn cancel_auction_test() {
//     let now = auction_args::AuctionArgsBuilder::get_now_u64();
//     let mut auction = auction::AuctionContract::deploy_with_default_args(true, now);
//     auction.cancel_auction(&auction.admin.clone(), now + 1001)
// }
//
// #[test]
// #[should_panic = "User(22)"]
// fn cancel_auction_after_bid_test() {
//     let now = auction_args::AuctionArgsBuilder::get_now_u64();
//     let mut auction = auction::AuctionContract::deploy_with_default_args(true, now);
//     auction.bid(&auction.bob.clone(), U512::from(40000), now + 1000);
//     auction.cancel_auction(&auction.admin.clone(), now + 1001)
// }
//
// #[test]
// fn cancel_auction_after_cancelled_bid_test() {
//     let now = auction_args::AuctionArgsBuilder::get_now_u64();
//     let mut auction = auction::AuctionContract::deploy_with_default_args(true, now);
//     auction.bid(&auction.bob.clone(), U512::from(40000), now + 1000);
//     auction.cancel_bid(&auction.bob.clone(), now + 1001);
//     auction.cancel_auction(&auction.admin.clone(), now + 1002)
// }
//
// #[test]
// #[should_panic = "User(6)"]
// fn english_auction_bidder_count_limit_test() {
//     let now = auction_args::AuctionArgsBuilder::get_now_u64();
//     let mut auction_args = auction_args::AuctionArgsBuilder::default();
//     auction_args.set_bidder_count_cap(Some(1));
//     let mut auction = auction::AuctionContract::deploy_contracts(auction_args);
//     auction.bid(&auction.bob.clone(), U512::from(30000), now + 1000);
//     auction.bid(&auction.ali.clone(), U512::from(40000), now + 1001);
//     auction.cancel_bid(&auction.bob.clone(), now + 1002);
//     auction.finalize(&auction.admin.clone(), now + 3500);
//     assert!(auction.is_finalized());
//     assert_eq!(auction.ali, auction.get_winner().unwrap());
//     assert_eq!(
//         U512::from(40000),
//         auction.get_winning_bid().unwrap()
//     );
// }
//
// #[test]
// fn english_increase_time_test() {
//     let now = auction_args::AuctionArgsBuilder::get_now_u64();
//     let mut auction_args = auction_args::AuctionArgsBuilder::default();
//     auction_args.set_auction_timer_extension(Some(10000));
//     let mut auction = auction::AuctionContract::deploy_contracts(auction_args);
//     assert_eq!(auction.get_end(), now + 4000);
//
//     auction.bid(&auction.bob.clone(), U512::from(30000), now + 1000);
//     assert_eq!(auction.get_end(), now + 14000);
//     auction.cancel_bid(&auction.bob.clone(), now + 1500);
//     auction.finalize(&auction.admin.clone(), now + 14000);
//     assert!(auction.is_finalized());
//     assert_eq!(None, auction.get_winner());
//     assert_eq!(None, auction.get_winning_bid());
// }
//
// #[test]
// fn english_auction_bid_step_test() {
//     let now = auction_args::AuctionArgsBuilder::get_now_u64();
//     let mut auction_args = auction_args::AuctionArgsBuilder::default();
//     auction_args.set_minimum_bid_step(Some(U512::from(10000)));
//     let mut auction = auction::AuctionContract::deploy_contracts(auction_args);
//     auction.bid(&auction.bob.clone(), U512::from(30000), now + 1000);
//     auction.bid(&auction.ali.clone(), U512::from(40000), now + 1001);
//     auction.finalize(&auction.admin.clone(), now + 4000);
//     assert!(auction.is_finalized());
//     assert_eq!(Some(auction.ali), auction.get_winner());
//     assert_eq!(Some(U512::from(40000)), auction.get_winning_bid());
// }
//
// #[test]
// #[should_panic = "User(3)"]
// fn english_auction_bid_step_test_failing() {
//     let now = auction_args::AuctionArgsBuilder::get_now_u64();
//     let mut auction_args = auction_args::AuctionArgsBuilder::default();
//     auction_args.set_minimum_bid_step(Some(U512::from(10001)));
//     let mut auction = auction::AuctionContract::deploy_contracts(auction_args);
//     auction.bid(&auction.bob.clone(), U512::from(30000), now + 1000);
//     auction.bid(&auction.ali.clone(), U512::from(40000), now + 1001);
// }
//
// #[test]
// fn marketplace_commission_test() {
//     let now = auction_args::AuctionArgsBuilder::get_now_u64();
//     let mut auction_args = auction_args::AuctionArgsBuilder::default();
//     let mut auction = auction::AuctionContract::deploy_contracts(auction_args);
//     auction.bid(
//         &auction.ali.clone(),
//         U512::from(100000),
//         now + 1000,
//     );
//     auction.finalize(&auction.admin.clone(), now + 4000);
//     assert!(auction.is_finalized());
//     // assert!(auction.get_marketplace_balance() >= U512::from(10000));
//     assert!(auction.get_marketplace_balance() >= U512::from(2500));
//     assert!(auction.get_comm_balance() > U512::from(0));
// }
//
// #[test]
// fn english_auction_bid_extend_finalize_test() {
//     let now = auction_args::AuctionArgsBuilder::get_now_u64();
//     let mut auction = auction::AuctionContract::deploy_with_default_args(true, now);
//     assert!(now < auction.get_end());
//     auction.extend_bid(&auction.bob.clone(), U512::from(30000), now);
//     auction.extend_bid(&auction.bob.clone(), U512::from(10000), now);
//     auction.finalize(&auction.admin.clone(), now + 3500);
//     assert!(auction.is_finalized());
//     assert_eq!(auction.bob, auction.get_winner().unwrap());
//     assert_eq!(
//         U512::from(40000),
//         auction.get_winning_bid().unwrap()
//     );
// }
//
// #[test]
// fn english_auction_bid_delta_finalize_test() {
//     let now = auction_args::AuctionArgsBuilder::get_now_u64();
//     let mut auction = auction::AuctionContract::deploy_with_default_args(true, now);
//     let bob = auction.bob;
//     assert!(now < auction.get_end());
//     println!("{}", auction.get_account_balance(&bob));
//     auction.delta_bid(&bob, U512::from(5_000_u64), now);
//     println!("{}", auction.get_account_balance(&bob));
//     auction.delta_bid(&bob, U512::from(8_000_u64), now);
//     println!("{}", auction.get_account_balance(&bob));
//     auction.finalize(&auction.admin.clone(), now + 3500);
//     assert!(auction.is_finalized());
//     assert_eq!(auction.bob, auction.get_winner().unwrap());
//     assert_eq!(
//         U512::from(8_000_u64),
//         auction.get_winning_bid().unwrap()
//     );
// }
