use std::collections::HashMap;

use crate::types::{AccountBalances, Transaction, TxId};

pub fn process_transaction(
    _txn: Transaction,
    _balances: &mut AccountBalances,
    _still_disputable_txns: &HashMap<TxId, Transaction>,
) {
    // process transaction
}
