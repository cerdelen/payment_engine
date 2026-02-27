use std::collections::HashMap;

use crate::txn_engine::{
    account_balance::{ClientAccountBalance, ClientId},
    transaction::Transaction,
};

pub type AccountBalances = HashMap<ClientId, ClientAccountBalance>;

#[derive(Debug, Default)]
pub struct TransactionEngine {
    balances: AccountBalances,
}

impl TransactionEngine {
    pub fn process_transaction(&mut self, _tx: Transaction) {}

    pub fn get_account_balances(&self) -> &AccountBalances {
        &self.balances
    }
}
