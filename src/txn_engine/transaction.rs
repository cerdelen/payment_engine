use serde::Deserialize;

use crate::txn_engine::{account::ClientId, amt::Amt};

pub type TxId = u32;

/// This struct represents an transaction event as input to be processed by the payment engine.
#[derive(Debug, Deserialize, Clone)]
#[allow(unused)]
pub struct TransactionInput {
    #[serde(rename = "type")]
    pub tx_type: TransactionType,
    #[serde(rename = "client")]
    pub client_id: ClientId,
    #[serde(rename = "tx")]
    pub tx_id: TxId,
    /// We will be scaling the amount values ourselves by the factor of 10 ^ 4
    #[serde(rename = "amount")]
    pub amt: Option<Amt>,
}

#[derive(Debug, Deserialize, PartialEq, Eq, Clone)]
#[serde(rename_all = "lowercase")]
pub enum TransactionType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

/// This struct represents an already processed valid former transaction.
#[derive(Debug)]
#[allow(unused)]
pub struct ProcessedTransaction {
    pub client_id: u16,
    pub amt: Amt,
    pub status: TransactionStatus,
}

impl ProcessedTransaction {
    /// Creates a new ProcessedTransaction struct with TransactionStatus 'Normal'.
    pub fn new(client_id: ClientId, amt: Amt) -> Self {
        Self {
            client_id,
            amt,
            status: TransactionStatus::Normal,
        }
    }
}

/// This struct tells us the current status of any given deposit or withdrawal transaction
#[derive(Debug, PartialEq, Eq)]
#[allow(unused)]
pub enum TransactionStatus {
    Normal,
    Disputed,
    ChargedBack,
}
