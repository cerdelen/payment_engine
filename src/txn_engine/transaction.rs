use serde::Deserialize;

use crate::txn_engine::amt::Amt;

pub type TxId = u32;

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct Transaction {
    #[serde(rename = "type")]
    pub tx_type: TransactionType,
    #[serde(rename = "client")]
    pub client_id: u16,
    #[serde(rename = "tx")]
    pub tx_id: TxId,
    /// We will be scaling the amount values ourselves by the factor of 10 ^ 4
    #[serde(rename = "amount")]
    pub amt: Option<Amt>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TransactionType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}
