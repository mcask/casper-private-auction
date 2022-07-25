use casper_types::{
    account::AccountHash, ContractPackageHash, Key, runtime_args,
    RuntimeArgs, U512,
};

use casper_private_auction_core::keys;

use crate::auction::BaseAuctionArgs;
use crate::utils::{base_account, get_now_u64};

#[derive(Debug)]
pub struct AuctionArgBuilder {
    // into Key
    beneficiary_account: AccountHash,
    // into Key
    token_contract_hash: ContractPackageHash,
    // into Key
    kyc_package_hash: ContractPackageHash,
    // into Key
    synth_package_hash: ContractPackageHash,
    token_id: String,
    start_time: u64,
    end_time: u64,
    name: String,
    swap_price: U512,
    nft_commission: u32,
}

impl AuctionArgBuilder {
    pub fn base(
        start_time: u64,
        swap_price: U512,
        nft_commission: u32,
    ) -> Self {
        let account = base_account();

        AuctionArgBuilder {
            beneficiary_account: account.clone(),
            token_contract_hash: ContractPackageHash::new([0u8; 32]),
            kyc_package_hash: ContractPackageHash::new([0u8; 32]),
            synth_package_hash: ContractPackageHash::new([0u8; 32]),
            token_id: "token_id".to_string(),
            start_time,
            end_time: start_time + 3500,
            name: "test".to_string(),
            swap_price,
            nft_commission,
        }
    }
}

impl BaseAuctionArgs for AuctionArgBuilder {
    fn build(&self) -> RuntimeArgs {
        runtime_args! {
            keys::BENEFICIARY_ACCOUNT=>Key::Account(self.beneficiary_account),
            keys::TOKEN_PACKAGE_HASH=>Key::Hash(self.token_contract_hash.value()),
            keys::KYC_PACKAGE_HASH=>Key::Hash(self.kyc_package_hash.value()),
            keys::SYNTHETIC_PACKAGE_HASH=>Key::Hash(self.synth_package_hash.value()),
            keys::TOKEN_ID=>self.token_id.to_owned(),
            keys::START => self.start_time,
            keys::END => self.end_time,
            keys::NAME => self.name.clone(),
            keys::SWAP_PRICE=> self.swap_price,
        }
    }

    fn set_start_time(&mut self, time: u64) {
        self.start_time = time;
    }

    fn set_end_time(&mut self, time: u64) {
        self.end_time = time;
    }

    fn set_swap_price(&mut self, price: U512) {
        self.swap_price = price;
    }

    fn set_beneficiary(&mut self, account: &AccountHash) {
        self.beneficiary_account = account.clone();
    }

    fn set_token_contract_hash(&mut self, hash: &ContractPackageHash) {
        self.token_contract_hash = hash.clone();
    }

    fn set_kyc_package_hash(&mut self, hash: &ContractPackageHash){
        self.kyc_package_hash = hash.clone();
    }

    fn set_synth_package_hash(&mut self, hash: &ContractPackageHash){
        self.synth_package_hash = hash.clone();
    }

    fn set_token_id(&mut self, token_id: &String){
        self.token_id = token_id.clone();
    }

    fn get_nft_commission(&self) -> u32 {
        self.nft_commission
    }

    fn get_wasm(&self) -> String {
        "swap-installer.wasm".to_string()
    }
}

impl Default for AuctionArgBuilder {
    fn default() -> Self {
        let account = base_account();
        let now: u64 = get_now_u64();
        AuctionArgBuilder {
            beneficiary_account: account.clone(),
            token_contract_hash: ContractPackageHash::new([0u8; 32]),
            kyc_package_hash: ContractPackageHash::new([0u8; 32]),
            synth_package_hash: ContractPackageHash::new([0u8; 32]),
            token_id: "token_id".to_string(),
            start_time: now + 500,
            end_time: now + 3500,
            name: "test".to_string(),
            swap_price: U512::from(2000),
            nft_commission: 100_u32
        }
    }
}
