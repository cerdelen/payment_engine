use serde::Deserialize;

use crate::txn_engine::amt::Amt;

pub type TxId = u32;

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct Transaction {
    #[serde(rename = "type")]
    tx_type: TransactionType,
    #[serde(rename = "client")]
    client_id: u16,
    #[serde(rename = "tx")]
    tx_id: TxId,
    /// We will be scaling the amount values ourselves by the factor of 10 ^ 4
    #[serde(rename = "amount")]
    amt: Amt,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransactionType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}
