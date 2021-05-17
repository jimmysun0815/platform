#ifndef wallet_mobile_ffi_h
#define wallet_mobile_ffi_h

/* Warning, this file is autogenerated by cbindgen. Don't modify this manually. */

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

typedef struct AuthenticatedKVLookup AuthenticatedKVLookup;

typedef struct FeeInputs FeeInputs;

typedef struct XfrKeyPair XfrKeyPair;

typedef struct XfrPublicKey XfrPublicKey;

/**
 * Returns the git commit hash and commit date of the commit this library was built against.
 */
char *findora_ffi_build_id(void);

char *findora_ffi_random_asset_type(void);

/**
 * Generates asset type as a Base64 string from a JSON-serialized JavaScript value.
 */
char *findora_ffi_asset_type_from_value(const char *code);

/**
 * Given a serialized state commitment and transaction, returns true if the transaction correctly
 * hashes up to the state commitment and false otherwise.
 * @param {string} state_commitment - String representing the state commitment.
 * @param {string} authenticated_txn - String representing the transaction.
 * @see {@link module:Network~Network#getTxn|Network.getTxn} for instructions on fetching a transaction from the ledger.
 * @see {@link module:Network~Network#getStateCommitment|Network.getStateCommitment}
 * for instructions on fetching a ledger state commitment.
 * @throws Will throw an error if the state commitment or the transaction fails to deserialize.
 */
bool findora_ffi_verify_authenticated_txn(const char *state_commitment,
                                          const char *authenticated_txn);

struct AuthenticatedKVLookup *findora_ffi_authenticated_kv_lookup_new(void);

/**
 * Given a serialized state commitment and an authenticated custom data result, returns true if the custom data result correctly
 * hashes up to the state commitment and false otherwise.
 * @param {string} state_commitment - String representing the state commitment.
 * @param {JsValue} authenticated_txn - JSON-encoded value representing the authenticated custom
 * data result.
 * @throws Will throw an error if the state commitment or the authenticated result fail to deserialize.
 */
bool findora_ffi_verify_authenticated_custom_data_result(const char *state_commitment,
                                                         const struct AuthenticatedKVLookup *authenticated_res);

uint64_t findora_ffi_calculate_fee(uint64_t ir_numerator,
                                   uint64_t ir_denominator,
                                   uint64_t outstanding_balance);

struct XfrPublicKey *findora_ffi_get_null_pk(void);

char *findora_ffi_create_default_policy_info(void);

char *findora_ffi_create_debt_policy_info(uint64_t ir_numerator,
                                          uint64_t ir_denominator,
                                          const char *fiat_code,
                                          uint64_t loan_amount);

char *findora_ffi_create_debt_memo(uint64_t ir_numerator,
                                   uint64_t ir_denominator,
                                   const char *fiat_code,
                                   uint64_t loan_amount);

/**
 * Generate mnemonic with custom length and language.
 * - @param `wordslen`: acceptable value are one of [ 12, 15, 18, 21, 24 ]
 * - @param `lang`: acceptable value are one of [ "en", "zh", "zh_traditional", "fr", "it", "ko", "sp", "jp" ]
 */
char *findora_ffi_generate_mnemonic_custom(uint8_t words_len,
                                           const char *lang);

char *findora_ffi_decryption_pbkdf2_aes256gcm(char *enc_key_pair, const char *password);

char *findora_ffi_encryption_pbkdf2_aes256gcm(const char *key_pair, const char *password);

/**
 * Constructs a transfer key pair from a hex-encoded string.
 * The encode a key pair, use `keypair_to_str` function.
 */
struct XfrKeyPair *findora_ffi_keypair_from_str(const char *key_pair_str);

/**
 * Returns bech32 encoded representation of an XfrPublicKey.
 */
char *findora_ffi_public_key_to_bech32(const struct XfrPublicKey *key);

/**
 * Extracts the public key as a string from a transfer key pair.
 */
char *findora_ffi_get_pub_key_str(const struct XfrKeyPair *key);

/**
 * Extracts the private key as a string from a transfer key pair.
 */
char *findora_ffi_get_priv_key_str(const struct XfrKeyPair *key);

/**
 * Restore the XfrKeyPair from a mnemonic with a default bip44-path,
 * that is "m/44'/917'/0'/0/0" ("m/44'/coin'/account'/change/address").
 */
struct XfrKeyPair *findora_ffi_restore_keypair_from_mnemonic_default(const char *phrase);

/**
 * Expresses a transfer key pair as a hex-encoded string.
 * To decode the string, use `keypair_from_str` function.
 */
char *findora_ffi_keypair_to_str(const struct XfrKeyPair *key_pair);

struct XfrKeyPair *findora_ffi_create_keypair_from_secret(const char *sk_str);

struct XfrPublicKey *findora_ffi_get_pk_from_keypair(const struct XfrKeyPair *key_pair);

/**
 * Creates a new transfer key pair.
 */
struct XfrKeyPair *findora_ffi_new_keypair(void);

char *findora_ffi_bech32_to_base64(const char *pk);

char *findora_ffi_base64_to_bech32(const char *pk);

void findora_ffi_fee_inputs_free(struct FeeInputs *ptr);

void findora_ffi_authenticated_kv_lookup_free(struct AuthenticatedKVLookup *ptr);

void findora_ffi_xfr_public_key_free(struct XfrPublicKey *ptr);

#endif /* wallet_mobile_ffi_h */