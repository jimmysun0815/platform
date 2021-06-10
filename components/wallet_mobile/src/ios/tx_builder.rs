use crate::rust::*;
use credentials::{CredIssuerPublicKey, CredUserPublicKey};
use std::os::raw::c_char;
use zei::xfr::sig::XfrKeyPair;

#[no_mangle]
pub extern "C" fn findora_ffi_transaction_builder_add_fee_relative_auto(
    builder: &TransactionBuilder,
    am: u64,
    kp: &XfrKeyPair,
) -> *mut TransactionBuilder {
    if let Ok(info) = builder.clone().add_fee_relative_auto(am, kp.clone()) {
        Box::into_raw(Box::new(info))
    } else {
        std::ptr::null_mut()
    }
}

/// Use this func to get the necessary infomations for generating `Relative Inputs`
#[no_mangle]
pub extern "C" fn findora_ffi_transaction_builder_get_relative_outputs(
    builder: &TransactionBuilder,
) -> safer_ffi::vec::Vec<ClientAssetRecord> {
    builder.clone().get_relative_outputs().into()
}

/// As the last operation of any transaction,
/// add a static fee to the transaction.
#[no_mangle]
pub extern "C" fn findora_ffi_transaction_builder_add_fee(
    builder: &TransactionBuilder,
    inputs: &FeeInputs,
) -> *mut TransactionBuilder {
    if let Ok(info) = builder.clone().add_fee(inputs.clone()) {
        Box::into_raw(Box::new(info))
    } else {
        std::ptr::null_mut()
    }
}

/// A simple fee checker for mainnet v1.0.
///
/// SEE [check_fee](ledger::data_model::Transaction::check_fee)
#[no_mangle]
pub extern "C" fn findora_ffi_transaction_builder_check_fee(
    builder: &TransactionBuilder,
) -> bool {
    builder.clone().check_fee()
}

/// Create a new transaction builder.
#[no_mangle]
pub extern "C" fn findora_ffi_transaction_builder_new(
    seq_id: u64,
) -> *mut TransactionBuilder {
    Box::into_raw(Box::new(TransactionBuilder::new(seq_id)))
}

/// Wraps around TransactionBuilder to add an asset definition operation to a transaction builder instance.
#[no_mangle]
pub extern "C" fn findora_ffi_transaction_builder_add_operation_create_asset(
    builder: &TransactionBuilder,
    key_pair: &XfrKeyPair,
    memo: *const c_char,
    token_code: *const c_char,
    asset_rules: &AssetRules,
) -> *mut TransactionBuilder {
    if let Ok(info) = builder.clone().add_operation_create_asset(
        key_pair,
        c_char_to_string(memo),
        c_char_to_string(token_code),
        asset_rules.clone(),
    ) {
        Box::into_raw(Box::new(info))
    } else {
        std::ptr::null_mut()
    }
}

/// Wraps around TransactionBuilder to add an asset issuance to a transaction builder instance.
///
/// Use this function for simple one-shot issuances.
#[no_mangle]
pub extern "C" fn findora_ffi_transaction_builder_add_basic_issue_asset(
    builder: &TransactionBuilder,
    key_pair: &XfrKeyPair,
    code: *const c_char,
    seq_num: u64,
    amount: u64,
    conf_amount: bool,
    zei_params: &PublicParams,
) -> *mut TransactionBuilder {
    if let Ok(info) = builder.clone().add_basic_issue_asset(
        key_pair,
        c_char_to_string(code),
        seq_num,
        amount,
        conf_amount,
        zei_params,
    ) {
        Box::into_raw(Box::new(info))
    } else {
        std::ptr::null_mut()
    }
}

/// Adds an operation to the transaction builder that appends a credential commitment to the address
/// identity registry.
#[no_mangle]
pub extern "C" fn findora_ffi_transaction_builder_add_operation_air_assign(
    builder: &TransactionBuilder,
    key_pair: &XfrKeyPair,
    user_public_key: &CredUserPublicKey,
    issuer_public_key: &CredIssuerPublicKey,
    commitment: &CredentialCommitment,
    pok: &CredentialPoK,
) -> *mut TransactionBuilder {
    if let Ok(info) = (*builder).clone().add_operation_air_assign(
        key_pair,
        user_public_key,
        issuer_public_key,
        commitment,
        pok,
    ) {
        Box::into_raw(Box::new(info))
    } else {
        std::ptr::null_mut()
    }
}

/// Adds an operation to the transaction builder that removes a hash from ledger's custom data
/// store.
#[no_mangle]
pub extern "C" fn findora_ffi_transaction_builder_add_operation_kv_update_no_hash(
    builder: &TransactionBuilder,
    auth_key_pair: &XfrKeyPair,
    key: &Key,
    seq_num: u64,
) -> *mut TransactionBuilder {
    if let Ok(info) =
        builder
            .clone()
            .add_operation_kv_update_no_hash(auth_key_pair, key, seq_num)
    {
        Box::into_raw(Box::new(info))
    } else {
        std::ptr::null_mut()
    }
}

/// Adds an operation to the transaction builder that adds a hash to the ledger's custom data
/// store.
#[no_mangle]
pub extern "C" fn findora_ffi_transaction_builder_add_operation_kv_update_with_hash(
    builder: &TransactionBuilder,
    auth_key_pair: &XfrKeyPair,
    key: &Key,
    seq_num: u64,
    kv_hash: &KVHash,
) -> *mut TransactionBuilder {
    if let Ok(info) = builder.clone().add_operation_kv_update_with_hash(
        auth_key_pair,
        key,
        seq_num,
        kv_hash,
    ) {
        Box::into_raw(Box::new(info))
    } else {
        std::ptr::null_mut()
    }
}

/// Adds an operation to the transaction builder that adds a hash to the ledger's custom data
/// store.
#[no_mangle]
pub extern "C" fn findora_ffi_transaction_builder_add_operation_update_memo(
    builder: &TransactionBuilder,
    auth_key_pair: &XfrKeyPair,
    code: *const c_char,
    new_memo: *const c_char,
) -> *mut TransactionBuilder {
    if let Ok(info) = builder.clone().add_operation_update_memo(
        auth_key_pair,
        c_char_to_string(code),
        c_char_to_string(new_memo),
    ) {
        Box::into_raw(Box::new(info))
    } else {
        std::ptr::null_mut()
    }
}

/// Adds a serialized transfer asset operation to a transaction builder instance.
#[no_mangle]
pub extern "C" fn findora_ffi_transaction_builder_add_transfer_operation(
    builder: &TransactionBuilder,
    op: *const c_char,
) -> *mut TransactionBuilder {
    if let Ok(info) = builder.clone().add_transfer_operation(c_char_to_string(op)) {
        Box::into_raw(Box::new(info))
    } else {
        std::ptr::null_mut()
    }
}

#[no_mangle]
pub extern "C" fn findora_ffi_transaction_builder_sign(
    builder: &TransactionBuilder,
    kp: &XfrKeyPair,
) -> *mut TransactionBuilder {
    if let Ok(info) = builder.clone().sign(kp) {
        Box::into_raw(Box::new(info))
    } else {
        std::ptr::null_mut()
    }
}

/// Extracts the serialized form of a transaction.
#[no_mangle]
pub extern "C" fn findora_ffi_transaction_builder_transaction(
    builder: &TransactionBuilder,
) -> *mut c_char {
    string_to_c_char(builder.transaction())
}

/// Calculates transaction handle.
#[no_mangle]
pub extern "C" fn findora_ffi_transaction_builder_transaction_handle(
    builder: &TransactionBuilder,
) -> *mut c_char {
    string_to_c_char(builder.transaction_handle())
}

/// Fetches a client record from a transaction.
/// @param {number} idx - Record to fetch. Records are added to the transaction builder sequentially.
#[no_mangle]
pub extern "C" fn findora_ffi_transaction_builder_get_owner_record(
    builder: &TransactionBuilder,
    idx: usize,
) -> *mut ClientAssetRecord {
    Box::into_raw(Box::new(builder.get_owner_record(idx)))
}

/// Fetches an owner memo from a transaction
/// @param {number} idx - Owner memo to fetch. Owner memos are added to the transaction builder sequentially.
#[no_mangle]
pub extern "C" fn findora_ffi_transaction_builder_get_owner_memo(
    builder: &TransactionBuilder,
    idx: usize,
) -> *mut OwnerMemo {
    if let Some(info) = builder.get_owner_memo(idx) {
        Box::into_raw(Box::new(info))
    } else {
        std::ptr::null_mut()
    }
}
