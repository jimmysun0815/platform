// Interface for issuing transactions that can be compiled to Wasm.
// Allows web clients to issue transactions from a browser contexts.
// For now, forwards transactions to a ledger hosted locally.
// To compile wasm package, run wasm-pack build in the wasm directory;
#![deny(warnings)]
use crate::wasm_data_model::*;
use core::fmt::Display;
use credentials::{
  credential_commit, credential_issuer_key_gen, credential_reveal, credential_sign,
  credential_user_key_gen, credential_verify, CredIssuerPublicKey, CredIssuerSecretKey,
  CredUserPublicKey, CredUserSecretKey, Credential as PlatformCredential,
};
use cryptohash::sha256;
use cryptohash::sha256::Digest as BitDigest;
use js_sys::Promise;
use ledger::data_model::{
  b64dec, b64enc, AssetRules, AssetTypeCode, AuthenticatedTransaction, Operation,
};
use ledger::policies::{DebtMemo, Fraction};
use rand_chacha::ChaChaRng;
use rand_core::SeedableRng;
use std::str;
use txn_builder::{
  BuildsTransactions, PolicyChoice, TransactionBuilder as PlatformTransactionBuilder,
  TransferOperationBuilder as PlatformTransferOperationBuilder,
};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, RequestMode};

use zei::serialization::ZeiFromToBytes;
use zei::xfr::asset_record::{open_blind_asset_record as open_bar, AssetRecordType};
use zei::xfr::lib::trace_assets as zei_trace_assets;
use zei::xfr::sig::{XfrKeyPair, XfrMultiSig, XfrPublicKey};
use zei::xfr::structs::{AssetRecordTemplate, AssetTracingPolicy, XfrBody, XfrNote};

mod wasm_data_model;

/////////// TRANSACTION BUILDING ////////////////

//Random Helpers

#[wasm_bindgen]
/// Generates random base64 encoded asset type string. Used in asset definitions.
/// @see {@link WasmTransactionBuilder#add_operation_create_asset} for instructions on how to define an asset with a new
/// asset type
pub fn random_asset_type() -> String {
  AssetTypeCode::gen_random().to_base64()
}

#[wasm_bindgen]
/// Given a serialized state commitment and transaction, returns true if the transaction correctly
/// hashes up to the state commitment and false otherwise.
/// @param {string} state_commitment - String representing the state commitment.
/// @param {string} authenticated_txn - String representing the transaction.
/// @see {@link get_transaction} for instructions on fetching a transaction from the ledger.
/// @see {@link get_state_commitment} for instructions on fetching a ledger state commitment.
/// @throws Will throw an error if the state commitment or the transaction fail to deserialize.
pub fn verify_authenticated_txn(state_commitment: String,
                                authenticated_txn: String)
                                -> Result<bool, JsValue> {
  let authenticated_txn = serde_json::from_str::<AuthenticatedTransaction>(&authenticated_txn)
        .map_err(|_e| JsValue::from_str("Could not deserialize transaction"))?;
  let state_commitment = serde_json::from_str::<BitDigest>(&state_commitment).map_err(|_e| {
                           JsValue::from_str("Could not deserialize state commitment")
                         })?;
  Ok(authenticated_txn.is_valid(state_commitment))
}

#[wasm_bindgen]
/// Performs a simple loan repayment fee calculation.
///
/// The returned fee is a fraction of the `outstanding_balance`
/// where the interest rate is expressed as a fraction `ir_numerator` / `ir_denominator`.
///
/// This function is specific to the  Lending Demo.
/// @param {BigInt} ir_numerator - Interest rate numerator.
/// @param {BigInt} ir_denominator - Interest rate denominator.
/// @param {BigInt} outstanding_balance - Amount of outstanding debt.
/// @ignore
pub fn calculate_fee(ir_numerator: u64, ir_denominator: u64, outstanding_balance: u64) -> u64 {
  ledger::policies::calculate_fee(outstanding_balance,
                                  Fraction::new(ir_numerator, ir_denominator))
}

#[wasm_bindgen]
/// Returns an address to use for cancelling debt tokens in a debt swap.
/// @ignore
pub fn get_null_pk() -> XfrPublicKey {
  XfrPublicKey::zei_from_bytes(&[0; 32])
}

#[wasm_bindgen]
pub fn create_default_policy_info() -> String {
  serde_json::to_string(&PolicyChoice::Fungible()).unwrap() // should never fail
}

#[wasm_bindgen]
/// Create policy information needed for debt token asset types.
/// This data will be parsed by the policy evalautor to ensure
/// that all payment and fee amounts are correct.
/// # Arguments
///
/// * `ir_numerator` - interest rate numerator
/// * `ir_denominator`- interest rate denominator
/// * `fiat_code` - base64 string representing asset type used to pay off the loan
/// * `amount` - loan amount
pub fn create_debt_policy_info(ir_numerator: u64,
                               ir_denominator: u64,
                               fiat_code: String,
                               loan_amount: u64)
                               -> Result<String, JsValue> {
  let fiat_code = AssetTypeCode::new_from_base64(&fiat_code).map_err(|_e| {
      JsValue::from_str("Could not deserialize asset token code")})?;

  serde_json::to_string(&PolicyChoice::LoanToken(Fraction::new(ir_numerator, ir_denominator),
    fiat_code, loan_amount))
      .map_err(|e| JsValue::from_str(&format!("Could not serialize PolicyChoice: {}",e)))
}

#[wasm_bindgen]
/// Creates the memo needed for debt token asset types. The memo will be parsed by the policy evaluator to ensure
/// that all payment and fee amounts are correct.
/// @param {BigInt} ir_numerator  - Interest rate numerator.
/// @param {BigInt} ir_denominator - Interest rate denominator.
/// @param {string} fiat_code - Base64 string representing asset type used to pay off the loan.
/// @param {BigInt} loan_amount - Loan amount.
/// @throws Will throw an error if `fiat_code` fails to deserialize.
/// @ignore
pub fn create_debt_memo(ir_numerator: u64,
                        ir_denominator: u64,
                        fiat_code: String,
                        loan_amount: u64)
                        -> Result<String, JsValue> {
  let fiat_code = AssetTypeCode::new_from_base64(&fiat_code)
        .map_err(|_e| JsValue::from_str("Could not deserialize asset token code"))?;
  let memo = DebtMemo { interest_rate: Fraction::new(ir_numerator, ir_denominator),
                        fiat_code,
                        loan_amount };
  Ok(serde_json::to_string(&memo).unwrap())
}

#[wasm_bindgen]
/// Structure that allows users to construct arbitrary transactions.
#[derive(Default)]
pub struct TransactionBuilder {
  transaction_builder: PlatformTransactionBuilder,
}

impl TransactionBuilder {
  pub fn get_builder(&self) -> &PlatformTransactionBuilder {
    &self.transaction_builder
  }

  pub fn get_builder_mut(&mut self) -> &mut PlatformTransactionBuilder {
    &mut self.transaction_builder
  }
}

#[wasm_bindgen]
impl TransactionBuilder {
  /// Create a new transaction builder.
  pub fn new() -> Self {
    Self::default()
  }

  /// Wraps around TransactionBuilder to add an asset definition operation to a transaction builder instance.
  ///
  /// @param {XfrKeyPair} key_pair -  Issuer XfrKeyPair.
  /// @param {string} memo - Text field for asset definition.
  /// @param {string} token_code - Optional Base64 string representing the token code of the asset to be issued
  /// If empty, a token code will be chosen at random.
  pub fn add_operation_create_asset(self,
                                    key_pair: &XfrKeyPair,
                                    memo: String,
                                    token_code: String)
                                    -> Result<TransactionBuilder, JsValue> {
    self.add_operation_create_asset_with_policy(key_pair,
                                                memo,
                                                token_code,
                                                create_default_policy_info())
  }

  pub fn add_operation_create_asset_with_policy(mut self,
                                                key_pair: &XfrKeyPair,
                                                memo: String,
                                                token_code: String,
                                                policy_choice: String)
                                                -> Result<TransactionBuilder, JsValue> {
    let asset_token = if token_code.is_empty() {
      AssetTypeCode::gen_random()
    } else {
      AssetTypeCode::new_from_base64(&token_code).unwrap()
    };

    let policy_choice = serde_json::from_str::<PolicyChoice>(&policy_choice).map_err(|e| {
                          JsValue::from_str(&format!("Could not deserialize PolicyChoice: {}", e))
                        })?;
    self.get_builder_mut()
        .add_operation_create_asset(&key_pair,
                                    Some(asset_token),
                                    AssetRules::default().set_traceable(true).clone(),
                                    &memo,
                                    policy_choice)
        .map_err(error_to_jsvalue)?;
    Ok(self)
  }

  pub fn add_policy_option(mut self,
                           token_code: String,
                           which_check: String)
                           -> Result<TransactionBuilder, JsValue> {
    let token_code = AssetTypeCode::new_from_base64(&token_code).map_err(|e| {
                       JsValue::from_str(&format!("Could not deserialize asset type code: {}", e))
                     })?;

    self.get_builder_mut()
        .add_policy_option(token_code, which_check);
    Ok(self)
  }

  /// Wraps around TransactionBuilder to add an asset issuance to a transaction builder instance.
  ///
  /// Use this function for simple one-shot issuances.
  ///
  /// @param {XfrKeyPair} key_pair  - Issuer XfrKeyPair.
  /// @param {AssetTracerKeyPair} tracer keypair - Optional tracking public key. Pass in tracing key or null. Used to decrypt amounts
  /// and types of traced assets.
  /// @param {string} code - Base64 string representing the token code of the asset to be issued.
  /// @param {BigInt} seq_num - Issuance sequence number. Every subsequent issuance of a given asset type must have a higher sequence number than before.
  /// @param {BigInt} amount - Amount to be issued.
  #[allow(clippy::too_many_arguments)]
  pub fn add_basic_issue_asset(mut self,
                               key_pair: &XfrKeyPair,
                               tracing_key: &AssetTracerKeyPair,
                               code: String,
                               seq_num: u64,
                               amount: u64,
                               conf_amount: bool)
                               -> Result<TransactionBuilder, JsValue> {
    let asset_token = AssetTypeCode::new_from_base64(&code)
             .map_err(|_e| JsValue::from_str("Could not deserialize asset token code"))?;

    // TODO: (keyao/noah) enable client support for identity
    // tracking?
    let tracing_policy = Some(AssetTracingPolicy { enc_keys: tracing_key.get_enc_key().clone(),
                                                   asset_tracking: true,
                                                   identity_tracking: None });

    let confidentiality_flags = AssetRecordType::from_booleans(conf_amount, false);
    self.get_builder_mut()
        .add_basic_issue_asset(&key_pair,
                               tracing_policy,
                               &asset_token,
                               seq_num,
                               amount,
                               confidentiality_flags)
        .map_err(error_to_jsvalue)?;
    Ok(self)
  }

  /// Adds an add air assign operation to a WasmTransactionBuilder instance.
  pub fn add_operation_air_assign(mut self,
                                  key_pair: &XfrKeyPair,
                                  issuer_public_key: &CredIssuerPublicKey,
                                  commitment: &CredentialCommitment)
                                  -> Result<TransactionBuilder, JsValue> {
    self.get_builder_mut()
        .add_operation_air_assign(key_pair,
                                  issuer_public_key.clone(),
                                  commitment.get_commitment_ref().clone(),
                                  commitment.get_pok_ref().clone())
        .map_err(error_to_jsvalue)?;
    Ok(self)
  }

  /// Adds a serialized operation to a WasmTransactionBuilder instance
  /// @param {string} op -  a JSON-serialized operation (i.e. a transfer operation).
  /// @see {@link WasmTransferOperationBuilder} for details on constructing a transfer operation.
  /// @throws Will throw an error if `op` fails to deserialize.
  pub fn add_operation(mut self, op: String) -> Result<TransactionBuilder, JsValue> {
    let op = serde_json::from_str::<Operation>(&op).map_err(error_to_jsvalue)?;
    self.get_builder_mut().add_operation(op);
    Ok(self)
  }

  pub fn sign(mut self, kp: &XfrKeyPair) -> Result<TransactionBuilder, JsValue> {
    self.get_builder_mut().sign(kp);
    Ok(self)
  }

  /// Extracts the serialized form of a transaction.
  // TODO Develop standard terminology for Javascript functions that may throw errors.
  pub fn transaction(&self) -> Result<String, JsValue> {
    Ok(self.get_builder()
           .serialize_str()
           .map_err(error_to_jsvalue)?)
  }

  /// Fetches a client record from a transaction.
  /// @param {number} idx - Record to fetch. Records are added to the transaction builder sequentially.
  pub fn get_owner_record(&self, idx: usize) -> Result<ClientAssetRecord, JsValue> {
    Ok(self.get_builder()
           .get_owner_record_and_memo(idx)
           .cloned()
           .map(|(output, _)| ClientAssetRecord { output })
           .ok_or_else(|| JsValue::from_str("Index out of range"))?)
  }

  /// Fetches an owner memo from a transaction
  /// @param {number} idx - Record to fetch. Records are added to the transaction builder sequentially.
  pub fn get_owner_memo(&self, idx: usize) -> Result<Option<OwnerMemo>, JsValue> {
    Ok(self.get_builder()
           .get_owner_record_and_memo(idx)
           .cloned()
           .map(|(_, memo)| (memo.map(|memo| OwnerMemo { memo })))
           .ok_or_else(|| JsValue::from_str("Index out of range"))?)
  }
}
#[wasm_bindgen]
#[derive(Default)]
/// Structure that enables clients to construct complex transfers.
pub struct TransferOperationBuilder {
  op_builder: PlatformTransferOperationBuilder,
}

impl TransferOperationBuilder {
  pub fn get_builder(&self) -> &PlatformTransferOperationBuilder {
    &self.op_builder
  }

  pub fn get_builder_mut(&mut self) -> &mut PlatformTransferOperationBuilder {
    &mut self.op_builder
  }
}

impl TransferOperationBuilder {
  pub fn add_input(mut self,
                   txo_ref: TxoRef,
                   asset_record: ClientAssetRecord,
                   owner_memo: Option<OwnerMemo>,
                   tracing_key: Option<&AssetTracerKeyPair>,
                   key: &XfrKeyPair,
                   amount: u64)
                   -> Result<TransferOperationBuilder, JsValue> {
    let oar =
      open_bar(asset_record.get_bar_ref(),
               &owner_memo.map(|memo| memo.get_memo_ref().clone()),
               key.get_sk_ref()).map_err(|_e| JsValue::from_str("Could not open asset record"))?;
    self.get_builder_mut()
        .add_input(*txo_ref.get_txo(),
                   oar,
                   tracing_key.map(|key| AssetTracingPolicy { enc_keys: key.get_enc_key()
                                                                           .clone(),
                                                              asset_tracking: true,
                                                              identity_tracking: None }),
                   amount)
        .map_err(error_to_jsvalue)?;
    Ok(self)
  }

  pub fn add_output(mut self,
                    amount: u64,
                    recipient: &XfrPublicKey,
                    tracing_key: Option<&AssetTracerKeyPair>,
                    code: String,
                    conf_amount: bool,
                    conf_type: bool)
                    -> Result<TransferOperationBuilder, JsValue> {
    let code = AssetTypeCode::new_from_base64(&code).map_err(error_to_jsvalue)?;

    let asset_record_type = AssetRecordType::from_booleans(conf_amount, conf_type);
    // TODO (noah/keyao) support identity tracing (issue #298)
    let template = if let Some(key) = tracing_key {
      AssetRecordTemplate::with_asset_tracking(amount,
                                               code.val,
                                               asset_record_type,
                                               *recipient,
                                               AssetTracingPolicy { enc_keys: key.get_enc_key()
                                                                                 .clone(),
                                                                    asset_tracking: true,
                                                                    identity_tracking: None })
    } else {
      AssetRecordTemplate::with_no_asset_tracking(amount, code.val, asset_record_type, *recipient)
    };
    self.get_builder_mut()
        .add_output(&template, None)
        .map_err(error_to_jsvalue)?;
    Ok(self)
  }
}

#[wasm_bindgen]
impl TransferOperationBuilder {
  /// Create a new transfer operation builder.
  pub fn new() -> Self {
    Self::default()
  }

  pub fn debug(&self) -> String {
    serde_json::to_string(&self.op_builder).unwrap()
  }

  /// Wraps around TransferOperationBuilder to add an input to a transfer operation builder.
  /// @param {TxoRef} txo_ref - Absolute or relative utxo reference
  /// @param {string} oar - Serialized opened asset record to serve as transfer input. This record must exist on the
  /// ledger for the transfer to be valid
  /// @param {BigInt} amount - Amount of input record to transfer
  /// @param tracing_key {AssetTracerKeyPair} - Tracing key, must be added to traceable
  /// assets.
  /// @see {@link create_absolute_txo_ref} or {@link create_relative_txo_ref} for details on txo
  /// references.
  /// @see {@link get_txo} for details on fetching blind asset records.
  /// @throws Will throw an error if `oar` or `txo_ref` fail to deserialize.
  pub fn add_input_with_tracking(self,
                                 txo_ref: TxoRef,
                                 asset_record: ClientAssetRecord,
                                 owner_memo: Option<OwnerMemo>,
                                 tracing_key: &AssetTracerKeyPair,
                                 key: &XfrKeyPair,
                                 amount: u64)
                                 -> Result<TransferOperationBuilder, JsValue> {
    self.add_input(txo_ref,
                   asset_record,
                   owner_memo,
                   Some(tracing_key),
                   key,
                   amount)
  }
  /// Wraps around TransferOperationBuilder to add an input to a transfer operation builder.
  /// @param {TxoRef} txo_ref - Absolute or relative utxo reference
  /// @param {string} oar - Serialized opened asset record to serve as transfer input. This record must exist on the
  /// ledger for the transfer to be valid
  /// @param {BigInt} amount - Amount of input record to transfer
  /// @see {@link create_absolute_txo_ref} or {@link create_relative_txo_ref} for details on txo
  /// references.
  /// @see {@link get_txo} for details on fetching blind asset records.
  /// @throws Will throw an error if `oar` or `txo_ref` fail to deserialize.
  // Note: these two functions are necessary because Wasm cannot handle optional references and I
  // don't want any of the functions to take ownership of the tracing key.
  pub fn add_input_no_tracking(self,
                               txo_ref: TxoRef,
                               asset_record: ClientAssetRecord,
                               owner_memo: Option<OwnerMemo>,
                               key: &XfrKeyPair,
                               amount: u64)
                               -> Result<TransferOperationBuilder, JsValue> {
    self.add_input(txo_ref, asset_record, owner_memo, None, key, amount)
  }

  /// Wraps around TransferOperationBuilder to add an output to a transfer operation builder.
  ///
  /// @param {BigInt} amount - amount to transfer to the recipient
  /// @param {XfrPublicKey} recipient - public key of the recipient
  /// @param code {string} - String representaiton of the asset token code
  /// @param conf_amount {bool} - Indicates whether output's amount is confidential
  /// @param conf_type {bool} - Indicates whether output's asset type is confidential
  /// @param tracing_key {AssetTracerKeyPair} - Optional tracing key, must be added to traced
  /// assets.
  /// @throws Will throw an error if `code` fails to deserialize.
  pub fn add_output_with_tracking(self,
                                  amount: u64,
                                  recipient: &XfrPublicKey,
                                  tracing_key: &AssetTracerKeyPair,
                                  code: String,
                                  conf_amount: bool,
                                  conf_type: bool)
                                  -> Result<TransferOperationBuilder, JsValue> {
    self.add_output(amount,
                    recipient,
                    Some(&tracing_key),
                    code,
                    conf_amount,
                    conf_type)
  }

  /// Wraps around TransferOperationBuilder to add an output to a transfer operation builder.
  ///
  /// @param {BigInt} amount - amount to transfer to the recipient
  /// @param {XfrPublicKey} recipient - public key of the recipient
  /// @param code {string} - String representaiton of the asset token code
  /// @param conf_amount {bool} - Indicates whether output's amount is confidential
  /// @param conf_type {bool} - Indicates whether output's asset type is confidential
  /// @throws Will throw an error if `code` fails to deserialize.
  pub fn add_output_without_tracking(self,
                                     amount: u64,
                                     recipient: &XfrPublicKey,
                                     code: String,
                                     conf_amount: bool,
                                     conf_type: bool)
                                     -> Result<TransferOperationBuilder, JsValue> {
    self.add_output(amount, recipient, None, code, conf_amount, conf_type)
  }

  /// Wraps around TransferOperationBuilder to ensure the transfer inputs and outputs are balanced.
  /// This function will add change outputs for all unspent portions of input records.
  /// @throws Will throw an error if the transaction cannot be balanced.
  pub fn balance(mut self) -> Result<TransferOperationBuilder, JsValue> {
    self.get_builder_mut()
        .balance()
        .map_err(|_e| JsValue::from_str("Error balancing txn"))?;
    Ok(self)
  }

  /// Wraps around TransferOperationBuilder to finalize the transaction.
  ///
  /// @param {TransferType} transfer_type - Transfer operation type.
  /// @throws Will throw an error if `transfer_type` fails to deserialize.
  /// @throws Will throw an error if input and output amounts do not add up.
  /// @throws Will throw an error if not all record owners have signed the transaction.
  pub fn create(mut self,
                transfer_type: TransferType)
                -> Result<TransferOperationBuilder, JsValue> {
    self.get_builder_mut()
        .create(*transfer_type.get_type())
        .map_err(error_to_jsvalue)?;
    Ok(self)
  }

  /// Wraps around TransferOperationBuilder to add a signature to the operation.
  ///
  /// All input owners must sign.
  ///
  /// @param {XfrKeyPair} kp - key pair of one of the input owners.
  pub fn sign(mut self, kp: &XfrKeyPair) -> Result<TransferOperationBuilder, JsValue> {
    self.get_builder_mut().sign(&kp).map_err(error_to_jsvalue)?;
    Ok(self)
  }

  pub fn builder(&self) -> String {
    serde_json::to_string(self.get_builder()).unwrap()
  }

  /// Wraps around TransferOperationBuilder to extract an operation expression as JSON.
  pub fn transaction(&self) -> Result<String, JsValue> {
    let op = self.get_builder().transaction().map_err(error_to_jsvalue)?;
    Ok(serde_json::to_string(&op).unwrap())
  }
}

///////////// CRYPTO //////////////////////
#[wasm_bindgen]
/// Returns a JsValue containing decrypted owner record information.
/// @param {ClientAssetRecord} record - Ownership record.
/// @param {OwnerMemo} owner_memo - Opening parameters.
/// @param {XfrKeyPair} key - Key of asset owner that is used to open the record.
pub fn open_client_asset_record(record: &ClientAssetRecord,
                                owner_memo: Option<OwnerMemo>,
                                key: &XfrKeyPair)
                                -> Result<JsValue, JsValue> {
  Ok(JsValue::from_serde(&open_bar(record.get_bar_ref(),
                             &owner_memo.map(|memo| memo.get_memo_ref().clone()),
                             key.get_sk_ref()).map_err(|_e| {
                                                JsValue::from_str("Could not open asset record")
                                              })?).unwrap())
}

#[wasm_bindgen]
/// Extracts the public key as a string from a transfer key pair.
pub fn get_pub_key_str(key_pair: &XfrKeyPair) -> String {
  serde_json::to_string(key_pair.get_pk_ref()).unwrap()
}
#[wasm_bindgen]
/// Extracts the private key as a string from a transfer key pair.
pub fn get_priv_key_str(key_pair: &XfrKeyPair) -> String {
  serde_json::to_string(key_pair.get_sk_ref()).unwrap()
}

#[wasm_bindgen]
/// Creates a new transfer key pair.
pub fn new_keypair() -> XfrKeyPair {
  let mut small_rng = rand::thread_rng();
  XfrKeyPair::generate(&mut small_rng)
}

#[wasm_bindgen]
/// Returns base64 encoded representation of an XfrPublicKey.
pub fn public_key_to_base64(key: &XfrPublicKey) -> String {
  b64enc(&XfrPublicKey::zei_to_bytes(&key))
}

#[wasm_bindgen]
/// Converts a base64 encoded public key string to a public key.
pub fn public_key_from_base64(key_pair: String) -> XfrPublicKey {
  XfrPublicKey::zei_from_bytes(&b64dec(&key_pair).unwrap())
}

#[wasm_bindgen]
/// Expresses a transfer key pair as a hex-encoded string.
/// To decode the string, use `keypair_from_str` function.
pub fn keypair_to_str(key_pair: &XfrKeyPair) -> String {
  hex::encode(key_pair.zei_to_bytes())
}

#[wasm_bindgen]
/// Constructs a transfer key pair from a hex-encoded string.
/// The encode a key pair, use `keypair_to_str` function.
pub fn keypair_from_str(str: String) -> XfrKeyPair {
  XfrKeyPair::zei_from_bytes(&hex::decode(str).unwrap())
}

#[wasm_bindgen]
/// Returns the SHA256 signature of the given string as a hex-encoded
/// string.
/// @ignore
pub fn sha256str(str: &str) -> String {
  let digest = sha256::hash(&str.as_bytes());
  hex::encode(digest)
}

#[wasm_bindgen]
/// Signs the given message using the given transfer key pair.
/// @ignore
pub fn sign(key_pair: &XfrKeyPair, message: String) -> Result<JsValue, JsValue> {
  let signature = key_pair.get_sk_ref()
                          .sign(&message.as_bytes(), key_pair.get_pk_ref());
  let mut smaller_signature: [u8; 32] = Default::default();
  smaller_signature.copy_from_slice(&signature.0.to_bytes()[0..32]);
  Ok(JsValue::from_serde(&smaller_signature).unwrap())
}
/*
fn u8_littleendian_slice_to_u32(array: &[u8]) -> u32 {
  u32::from(array[0])
  | u32::from(array[1]) << 8
  | u32::from(array[2]) << 16
  | u32::from(array[3]) << 24
}

fn u32_pair_to_u64(x: (u32, u32)) -> u64 {
  (x.1 as u64) << 32 ^ (x.0 as u64)
}
*/
/*
#[wasm_bindgen]
pub fn get_tracked_amount(blind_asset_record: String,
                          issuer_private_key_point: String)
                          -> Result<String, JsValue> {
  let pc_gens = PedersenGens::default();
  let blind_asset_record = serde_json::from_str::<BlindAssetRecord>(&blind_asset_record).map_err(|_e| {
                             JsValue::from_str("Could not deserialize blind asset record")
                           })?;
  let issuer_private_key = serde_json::from_str(&issuer_private_key_point).map_err(|_e| {
                             JsValue::from_str("Could not deserialize issuer private key")
                           })?;
  if let Some(lock_amount) = blind_asset_record.issuer_lock_amount {
    match (elgamal_decrypt(&RistrettoPoint(pc_gens.B), &(lock_amount.0), &issuer_private_key),
           elgamal_decrypt(&RistrettoPoint(pc_gens.B), &(lock_amount.1), &issuer_private_key))
    {
      (Ok(s1), Ok(s2)) => {
        let amount = u32_pair_to_u64((u8_littleendian_slice_to_u32(s1.0.as_bytes()),
                                      u8_littleendian_slice_to_u32(s2.0.as_bytes())));
        Ok(amount.to_string())
      }
      (_, _) => Err(JsValue::from_str("Unable to decrypt amount")),
    }
  } else {
    Err(JsValue::from_str("Asset record does not contain decrypted lock amount"))
  }
}
*/

// Ensures that the transaction serialization is valid URI text
#[wasm_bindgen]
/// Submit a transaction to the ledger and return a promise for the
/// ledger's eventual response. The transaction will be enqueued for
/// validation. If it is valid, it will eventually be committed to the
/// ledger.
///
/// To determine whether or not the transaction has been committed to the ledger,
/// query the ledger by transaction handle.
///
/// Contained in the response of `submit_transaction` is a `TransactionHandle` that can be used to
/// query the status of the transaction.
/// @param {string} path - Submission server path (e.g. `https://localhost:8669`)
/// @param {transaction_str} - JSON-encoded transaction string.
///
/// @see {@link get_txn_status} for information about transaction statuses.
// TODO Design and implement a notification mechanism.
pub fn submit_transaction(path: String, transaction_str: String) -> Result<Promise, JsValue> {
  let mut opts = RequestInit::new();
  opts.method("POST");
  opts.mode(RequestMode::Cors);
  opts.body(Some(&JsValue::from_str(&transaction_str)));

  let req_string = format!("{}/submit_transaction", path);

  create_query_promise(&opts, &req_string, true)
}

#[wasm_bindgen]
/// Given a transaction ID, returns a promise for the transaction status.
pub fn get_txn_status(path: String, handle: String) -> Result<Promise, JsValue> {
  let mut opts = RequestInit::new();
  opts.method("GET");
  opts.mode(RequestMode::Cors);

  let req_string = format!("{}/txn_status/{}", path, handle);

  create_query_promise(&opts, &req_string, false)
}

#[wasm_bindgen]
/// If successful, returns a promise that will eventually provide a
/// JsValue describing an unspent transaction output (UTXO).
/// Otherwise, returns 'not found'. The request fails if the txo uid
/// has been spent or the transaction index does not correspond to a
/// transaction.
/// @param {string} path - Address of ledger server.
/// @param {BigInt} index - UTXO index.
// TODO Provide an example (test case) that demonstrates how to
// handle the error in the case of an invalid transaction index.
// TODO Rename this function get_utxo
pub fn get_txo(path: String, index: u64) -> Result<Promise, JsValue> {
  let mut opts = RequestInit::new();
  opts.method("GET");
  opts.mode(RequestMode::Cors);

  let req_string = format!("{}/utxo_sid/{}", path, format!("{}", index));

  create_query_promise(&opts, &req_string, false)
}

#[wasm_bindgen]
/// If successful, returns a promise that will eventually provide a
/// JsValue describing a transaction.
/// Otherwise, returns 'not found'. The request fails if the transaction index does not correspond
/// to a transaction.
///
/// @param {String} path - Ledger server path.
/// @param {BigInt} index - Transaction index.
///
// TODO Provide an example (test case) that demonstrates how to
// handle the error in the case of an invalid transaction index.
pub fn get_transaction(path: String, index: u64) -> Result<Promise, JsValue> {
  let mut opts = RequestInit::new();
  opts.method("GET");
  opts.mode(RequestMode::Cors);

  let req_string = format!("{}/txn_sid/{}", path, format!("{}", index));

  create_query_promise(&opts, &req_string, false)
}

#[wasm_bindgen]
/// Returns a JSON-encoded version of the state commitment of a running ledger. This is used to
/// check the authenticity of transactions and blocks.
pub fn get_state_commitment(path: String) -> Result<Promise, JsValue> {
  let mut opts = RequestInit::new();
  opts.method("GET");
  opts.mode(RequestMode::Cors);

  let req_string = format!("{}/state_commitment", path);

  create_query_promise(&opts, &req_string, false)
}

#[wasm_bindgen]
/// If successful, returns a promise that will eventually provide a
/// JsValue describing an asset token. Otherwise, returns 'not found'.
/// The request fails if the given asset name does not correspond to
/// an asset.
/// @param {string} path: Address of ledger server.
/// @param {string} name: Base64-encoded asset token string.
///
// TODO Provide an example (test case) that demonstrates how to
// handle the error in the case of an undefined asset.
pub fn get_asset_token(path: String, name: String) -> Result<Promise, JsValue> {
  let mut opts = RequestInit::new();
  opts.method("GET");
  opts.mode(RequestMode::Cors);

  let req_string = format!("{}/asset_token/{}", path, name);

  create_query_promise(&opts, &req_string, false)
}

// Given a request string and a request init object, constructs
// the JS promise to be returned to the client.
/// @ignore
fn create_query_promise(opts: &RequestInit,
                        req_string: &str,
                        is_json: bool)
                        -> Result<Promise, JsValue> {
  let request = Request::new_with_str_and_init(&req_string, &opts)?;
  if is_json {
    request.headers().set("content-type", "application/json")?;
  }
  let window = web_sys::window().unwrap();
  let request_promise = window.fetch_with_request(&request);
  Ok(future_to_promise(JsFuture::from(request_promise)))
}

/// Generates a new credential issuer key.
/// @param {JsValue} attributes: Array of attribute types of the form `[{name: "credit_score",
/// size: 3}]'. The size refers to byte-size of the credential. In this case, the "credit_score"
/// attribute is represented as a 3 byte string "760". `attributes` is the list of attribute types
/// that the issuer can sign off on.
#[wasm_bindgen]
pub fn wasm_credential_issuer_key_gen(attributes: JsValue) -> CredentialIssuerKeyPair {
  let mut prng = ChaChaRng::from_entropy();
  let mut attributes: Vec<AttributeDefinition> = attributes.into_serde().unwrap();
  let attributes: Vec<(String, usize)> = attributes.drain(..)
                                                   .map(|attr| (attr.name, attr.size))
                                                   .collect();

  let (pk, sk) = credential_issuer_key_gen(&mut prng, &attributes[..]);
  CredentialIssuerKeyPair { pk, sk }
}

/// Generates a new credential user key.
/// @param {CredIssuerPublicKey} issuer_pub_key - The credential issuer that can sign off on this
/// user's attributes.
#[wasm_bindgen]
pub fn wasm_credential_user_key_gen(issuer_pub_key: &CredIssuerPublicKey) -> CredentialUserKeyPair {
  let mut prng = ChaChaRng::from_entropy();
  let (pk, sk) = credential_user_key_gen(&mut prng, issuer_pub_key);
  CredentialUserKeyPair { pk, sk }
}

fn error_to_jsvalue<T: Display>(e: T) -> JsValue {
  JsValue::from_str(&format!("{}", e))
}

/// Generates a signature on user attributes that can be used to create a credential.
/// @param {CredIssuerSecretKey} issuer_secret_key - Secret key of credential issuer.
/// @param {CredUserPublicKey} user_public_key - Public key of credential user.
/// @param {JsValue} attributes - Array of attribute assignments of the form `[{name: "credit_score",
/// val: "760"}]'.
/// @throws Will throw an error if the signature cannot be generated.
#[wasm_bindgen]
pub fn wasm_credential_sign(issuer_secret_key: &CredIssuerSecretKey,
                            user_public_key: &CredUserPublicKey,
                            attributes: JsValue)
                            -> Result<CredentialSignature, JsValue> {
  let mut prng = ChaChaRng::from_entropy();
  let attributes: Vec<AttributeAssignment> = attributes.into_serde().map_err(|_e| JsValue::from("Could not deserialize attributes. Please ensure that attribute definition is of the form [{name: string, val: string}]"))?;
  let attributes: Vec<(String, &[u8])> =
    attributes.iter()
              .map(|attr| (attr.name.clone(), attr.val.as_bytes()))
              .collect();
  let sig = credential_sign(&mut prng, &issuer_secret_key, &user_public_key, &attributes)
    .map_err(error_to_jsvalue)?;
  Ok(CredentialSignature { sig })
}

/// Generates a signature on user attributes that can be used to create a credential.
/// @param {CredIssuerPublicKey} issuer_public_key - Public key of credential issuer.
/// @param {CredentialSignature} signature - Credential issuer signature on attributes.
/// @param {JsValue} attributes - Array of attribute assignments of the form `[{name: "credit_score",
/// val: "760"}]'.
#[wasm_bindgen]
pub fn create_credential(issuer_public_key: &CredIssuerPublicKey,
                         signature: &CredentialSignature,
                         attributes: &JsValue)
                         -> Credential {
  let attributes: Vec<AttributeAssignment> = attributes.into_serde().unwrap();
  let attributes: Vec<(String, Vec<u8>)> =
    attributes.iter()
              .map(|attr| (attr.name.clone(), attr.val.as_bytes().to_vec()))
              .collect();
  Credential { credential: PlatformCredential { attributes,
                                                issuer_pub_key: issuer_public_key.clone(),
                                                signature: signature.get_sig_ref().clone() } }
}

/// Generates a credential commitment. A credential commitment can be used to selectively reveal
/// attribute assignments.
/// @param {CredUserSecretKey} user_secret_key - Secret key of credential user.
/// @param {XfrPublicKey} user_public_key - Ledger signing key to link this credential to.
/// @param {Credential} credential - Credential object.
#[wasm_bindgen]
pub fn wasm_credential_commit(user_secret_key: &CredUserSecretKey,
                              user_public_key: &XfrPublicKey,
                              credential: &Credential)
                              -> Result<CredentialCommitment, JsValue> {
  let mut prng = ChaChaRng::from_entropy();
  let (commitment, pok, _key) =
    credential_commit(&mut prng,
                      &user_secret_key,
                      credential.get_cred_ref(),
                      &user_public_key.as_bytes()).map_err(error_to_jsvalue)?;
  Ok(CredentialCommitment { commitment, pok })
}

/// Selectively reveals attributes committed to in a credential commitment
/// @param {CredUserSecretKey} user_sk - Secret key of credential user.
/// @param {Credential} credential - Credential object.
/// @param {JsValue} reveal_fields - Array of string names representing credentials to reveal (i.e.
/// `["credit_score"]`).
#[wasm_bindgen]
pub fn wasm_credential_reveal(user_sk: &CredUserSecretKey,
                              credential: &Credential,
                              reveal_fields: JsValue)
                              -> Result<CredentialRevealSig, JsValue> {
  let mut prng = ChaChaRng::from_entropy();
  let reveal_fields: Vec<String> = reveal_fields.into_serde().unwrap();
  Ok(CredentialRevealSig { sig: credential_reveal(&mut prng,
                                                  &user_sk,
                                                  credential.get_cred_ref(),
                                                  &reveal_fields[..]).map_err(|e| {
                                                                       error_to_jsvalue(e)
                                                                     })? })
}

/// Verifies revealed attributes from a commitment.
/// @param {CredIssuerPublicKey} issuer_pub_key - Public key of credential issuer.
/// @param {JsValue} reveal_fields - Array of string names representing credentials to reveal (i.e.
/// @param {JsValue} attributes - Array of attribute assignments to check of the form `[{name: "credit_score",
/// val: "760"}]'.
/// `["credit_score"]`).
/// @param {CredentialRevealSig} reveal_sig - Credential reveal signature.
#[wasm_bindgen]
pub fn wasm_credential_verify(issuer_pub_key: &CredIssuerPublicKey,
                              attributes: JsValue,
                              reveal_sig: &CredentialRevealSig)
                              -> Result<(), JsValue> {
  let attributes: Vec<AttributeAssignment> = attributes.into_serde().unwrap();
  let attributes: Vec<(String, &[u8])> =
    attributes.iter()
              .map(|attr| (attr.name.clone(), attr.val.as_bytes()))
              .collect();
  credential_verify(issuer_pub_key,
                    &attributes,
                    &reveal_sig.get_sig_ref().sig_commitment,
                    &reveal_sig.get_sig_ref().pok).map_err(error_to_jsvalue)?;
  Ok(())
}

// Asset Tracing

#[wasm_bindgen]
/// Returns information about traceable assets for a given transfer.
/// @param {JsValue} xfr_note - JSON of a transfer note from a transfer operation.
/// @param {AssetTracerKeyPair} - Asset tracer keypair.
/// @param {JsValue} candidate_assets - List of asset types traced by the tracer keypair.
pub fn trace_assets(xfr_body: JsValue,
                    tracer_keypair: &AssetTracerKeyPair,
                    candidate_assets: JsValue)
                    -> Result<JsValue, JsValue> {
  let candidate_assets: Vec<String> = candidate_assets.into_serde().map_err(error_to_jsvalue)?;
  let xfr_body: XfrBody = xfr_body.into_serde().map_err(error_to_jsvalue)?;
  let candidate_assets: Vec<[u8; 16]> =
    candidate_assets.iter()
                    .map(|asset_type_str| {
                      AssetTypeCode::new_from_str(&asset_type_str.to_string()).val
                    })
                    .collect();
  let record_data = zei_trace_assets(&XfrNote { body: xfr_body,
                                                multisig: XfrMultiSig::default() },
                                     tracer_keypair.get_keys(),
                                     &candidate_assets).map_err(error_to_jsvalue)?;
  let record_data: Vec<(u64, String)> = record_data.iter()
                                                   .map(|(amt, asset_type, _, _)| {
                                                     let asset_type_code =
                                                       AssetTypeCode { val: *asset_type };
                                                     (*amt, asset_type_code.to_base64())
                                                   })
                                                   .collect();
  Ok(JsValue::from_serde(&record_data).unwrap())
}

#[test]
pub fn test() {
  let kp = new_keypair();
  let b64 = public_key_to_base64(kp.get_pk_ref());
  let pk = public_key_from_base64(b64);
  dbg!(pk);
}
