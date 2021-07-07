//! Smart address operation for transaction.

use crate::data_model::NoReplayToken;
use crate::data_model::{Operation, Transaction, ASSET_TYPE_FRA, BLACK_HOLE_PUBKEY};
use ruc::*;
use serde::{Deserialize, Serialize};
use zei::xfr::sig::{XfrKeyPair, XfrPublicKey, XfrSignature};
use zei::xfr::structs::{XfrAmount, XfrAssetType};

/// Use this operation to transfer.
///
/// This operation only support binded xfr_address is sender address.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ConvertAccount {
    pub data: Data,
    pub public: XfrPublicKey,
    pub signature: XfrSignature,
}

impl ConvertAccount {
    pub fn new(keypair: &XfrKeyPair, nonce: NoReplayToken) -> Self {
        let data = Data::new(nonce);
        let public = keypair.get_pk();
        let signature = keypair.sign(&data.to_bytes());
        Self {
            data,
            public,
            signature,
        }
    }

    pub fn verify(&self) -> Result<()> {
        self.public
            .verify(&self.data.to_bytes(), &self.signature)
            .c(d!())
    }

    //     pub fn check_by_tx(&self, tx: &Transaction) -> Result<u64> {
    // check_convert_tx_amount(tx)
    // }

    pub fn set_nonce(&mut self, nonce: NoReplayToken) {
        self.data.nonce = nonce;
    }

    pub fn get_nonce(&self) -> NoReplayToken {
        self.data.nonce
    }

    pub fn get_related_address(&self) -> XfrPublicKey {
        self.public
    }
}

/// The body of TranserToAccount.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Data {
    pub nonce: NoReplayToken,
}

impl Data {
    pub fn new(nonce: NoReplayToken) -> Self {
        Data { nonce }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        pnk!(bincode::serialize(self))
    }
}

pub fn check_convert_tx_amount(tx: &Transaction) -> Result<(u64, XfrPublicKey)> {
    let mut owner = None;

    let mut amount = 0;

    for op in &tx.body.operations {
        if let Operation::ConvertAccount(ca) = op {
            if owner.is_some() {
                return Err(eg!("tx must have 1 convert account"));
            }
            owner = Some(ca.public)
        }
        if let Operation::TransferAsset(t) = op {
            for o in &t.body.outputs {
                if matches!(o.record.asset_type, XfrAssetType::Confidential(_))
                    || matches!(o.record.amount, XfrAmount::Confidential(_))
                {
                    return Err(eg!("convert can't support "));
                }
                if let XfrAssetType::NonConfidential(ty) = o.record.asset_type {
                    if ty == ASSET_TYPE_FRA && o.record.public_key == *BLACK_HOLE_PUBKEY
                    {
                        if let XfrAmount::NonConfidential(i_am) = o.record.amount {
                            amount += i_am;
                        }
                    }
                }
            }
        }
    }
    if owner.is_none() {
        return Err(eg!("this tx isn't a convert tx"));
    }
    Ok((amount, owner.unwrap()))
}