use std::path::PathBuf;
use blake2::digest::VariableOutput;
use blake2::VarBlake2b;
use blake2::digest::Update;

use casper_engine_test_support::{
    DeployItemBuilder, ExecuteRequestBuilder, InMemoryWasmTestBuilder, ARG_AMOUNT,
    DEFAULT_ACCOUNT_ADDR, DEFAULT_PAYMENT,
};
use casper_execution_engine::core::engine_state::ExecuteRequest;
use casper_types::{account::AccountHash, bytesrepr::FromBytes, runtime_args, system::mint, CLTyped, ContractHash, ContractPackageHash, Key, RuntimeArgs, StoredValue, U512, SecretKey, PublicKey};
use casper_types::bytesrepr::ToBytes;
use rand::Rng;

pub fn base_account() -> AccountHash {
    let key = SecretKey::ed25519_from_bytes([1u8; 32]).unwrap();
    let pk = PublicKey::from(&key);
    pk.to_account_hash()
}

pub fn create_account() -> AccountHash {
    let key = SecretKey::ed25519_from_bytes(rand::thread_rng().gen::<[u8; 32]>()).unwrap();
    let pk = PublicKey::from(&key);
    pk.to_account_hash()
}

pub fn key_and_value_to_str<T: CLTyped + ToBytes>(key: &Key, value: &T) -> String {
    let mut hasher = VarBlake2b::new(32).unwrap();
    hasher.update(key.to_bytes().unwrap());
    hasher.update(value.to_bytes().unwrap());
    let mut ret = [0u8; 32];
    hasher.finalize_variable(|hash| ret.clone_from_slice(hash));
    hex::encode(ret)
}

pub fn query<T: FromBytes + CLTyped>(
    builder: &InMemoryWasmTestBuilder,
    base: Key,
    path: &[String],
) -> T {
    builder
        .query(None, base, path)
        .expect("should be stored value.")
        .as_cl_value()
        .expect("should be cl value.")
        .clone()
        .into_t()
        .expect("Wrong type in query result.")
}

pub fn fund_account(account: &AccountHash, amount: U512) -> ExecuteRequest {
    let deploy_item = DeployItemBuilder::new()
        .with_address(*DEFAULT_ACCOUNT_ADDR)
        .with_authorization_keys(&[*DEFAULT_ACCOUNT_ADDR])
        .with_empty_payment_bytes(runtime_args! {ARG_AMOUNT => *DEFAULT_PAYMENT})
        .with_transfer_args(runtime_args! {
            mint::ARG_AMOUNT => amount,
            mint::ARG_TARGET => *account,
            mint::ARG_ID => <Option::<u64>>::None
        })
        .with_deploy_hash(rand::thread_rng().gen())
        .build();

    ExecuteRequestBuilder::from_deploy_item(deploy_item).build()
}
//
// pub fn empty_account(account: &AccountHash) -> ExecuteRequest {
//     let deploy_item = DeployItemBuilder::new()
//         .with_address(*DEFAULT_ACCOUNT_ADDR)
//         .with_authorization_keys(&[*DEFAULT_ACCOUNT_ADDR])
//         .with_empty_payment_bytes(runtime_args! {ARG_AMOUNT => *DEFAULT_PAYMENT})
//         .with_transfer_args(runtime_args! {
//             mint::ARG_AMOUNT => U512::from(1_u64),
//             mint::ARG_TARGET => *account,
//             mint::ARG_ID => <Option::<u64>>::None
//         })
//         .with_deploy_hash(rand::thread_rng().gen())
//         .build();
//
//     ExecuteRequestBuilder::from_deploy_item(deploy_item).build()
// }

pub enum DeploySource {
    Code(PathBuf),
    ByContractHash {
        hash: ContractHash,
        method: String,
    },
    ByPackageHash {
        package_hash: ContractPackageHash,
        method: String,
    },
}

pub fn deploy(
    builder: &mut InMemoryWasmTestBuilder,
    deployer: &AccountHash,
    source: &DeploySource,
    args: RuntimeArgs,
    success: bool,
    block_time: Option<u64>,
) {
    // let deploy_hash = rng.gen();
    let mut deploy_builder = DeployItemBuilder::new()
        .with_empty_payment_bytes(runtime_args! {ARG_AMOUNT => *DEFAULT_PAYMENT})
        .with_address(*deployer)
        .with_authorization_keys(&[*deployer])
        .with_deploy_hash(rand::thread_rng().gen());

    deploy_builder = match source {
        DeploySource::Code(path) => deploy_builder.with_session_code(path, args),
        DeploySource::ByContractHash { hash, method } => {
            deploy_builder.with_stored_session_hash(*hash, method, args)
        }
        DeploySource::ByPackageHash {
            package_hash,
            method,
        } => deploy_builder.with_stored_versioned_contract_by_hash(
            package_hash.value(),
            None,
            method,
            args,
        ),
    };

    let mut execute_request_builder =
        ExecuteRequestBuilder::from_deploy_item(deploy_builder.build());
    if let Some(ustamp) = block_time {
        execute_request_builder = execute_request_builder.with_block_time(ustamp)
    }
    let exec = builder.exec(execute_request_builder.build());
    if success {
        exec.expect_success()
    } else {
        exec.expect_failure()
    }
    .commit();
}

pub fn query_dictionary_item(
    builder: &InMemoryWasmTestBuilder,
    key: Key,
    dictionary_name: Option<String>,
    dictionary_item_key: String,
) -> Result<StoredValue, String> {
    let empty_path = vec![];
    let dictionary_key_bytes = dictionary_item_key.as_bytes();
    let address = match key {
        Key::Account(_) | Key::Hash(_) => {
            if let Some(name) = dictionary_name {
                let stored_value = builder.query(None, key, &[])?;

                let named_keys = match &stored_value {
                    StoredValue::Account(account) => account.named_keys(),
                    StoredValue::Contract(contract) => contract.named_keys(),
                    _ => {
                        return Err(
                            "Provided base key is nether an account or a contract".to_string()
                        )
                    }
                };

                let dictionary_uref = named_keys
                    .get(&name)
                    .and_then(Key::as_uref)
                    .ok_or_else(|| "No dictionary uref was found in named keys".to_string())?;

                Key::dictionary(*dictionary_uref, dictionary_key_bytes)
            } else {
                return Err("No dictionary name was provided".to_string());
            }
        }
        Key::URef(uref) => Key::dictionary(uref, dictionary_key_bytes),
        Key::Dictionary(address) => Key::Dictionary(address),
        _ => return Err("Unsupported key type for a query to a dictionary item".to_string()),
    };
    builder.query(None, address, &empty_path)
}

pub fn get_now_u64() -> u64 {
    use std::time::SystemTime;
    match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(n) => n.as_millis() as u64,
        Err(_) => panic!("SystemTime before UNIX EPOCH!"),
    }
}