use std::collections::HashMap;

use crate::txn_engine::{
    account::{ClientAccount, ClientId},
    transaction::{Transaction, TransactionType},
};

pub type AccountBalances = HashMap<ClientId, ClientAccount>;

#[derive(Debug, Default)]
pub struct TransactionEngine {
    balances: AccountBalances,
}

impl TransactionEngine {
    pub fn process_transaction(&mut self, tx: Transaction) {
        match tx.tx_type {
            TransactionType::Deposit => self.handle_deposit(tx),
            TransactionType::Withdrawal => self.handle_withdrawal(tx),
            TransactionType::Dispute => self.handle_dispute(tx),
            TransactionType::Resolve => self.handle_resolve(tx),
            TransactionType::Chargeback => self.handle_chargeback(tx),
        }
    }


    /// Returns a reference to the current account balances of this [`TransactionEngine`].
    pub fn get_account_balances(&self) -> &AccountBalances {
        &self.balances
    }

    fn handle_deposit(&mut self, tx: Transaction) {
        let res = if let Some(amt) = tx.amt {
            let account = self.balances.entry(tx.client_id).or_insert(ClientAccount::new(tx.client_id));
            account.deposit(amt)
        } else {
            Err("deposit transaction is missing an amount")
        };

        if let Err(e) = res {
            eprintln!("Error: Deposit for {} failed: {e}", tx.client_id);
        }
    }

    fn handle_withdrawal(&mut self, tx: Transaction) {
        let res = if let Some(amt) = tx.amt {
            let account = self.balances.entry(tx.client_id).or_insert(ClientAccount::new(tx.client_id));
            account.withdraw(amt)
        } else {
            Err("withdrawal transaction is missing an amount")
        };

        if let Err(e) = res {
            eprintln!("Error: Withdrawal for {} failed: {e}", tx.client_id);
        }
    }

    fn handle_dispute(&mut self, tx: Transaction) {
    }

    fn handle_resolve(&mut self, tx: Transaction) {
    }

    fn handle_chargeback(&mut self, tx: Transaction) {
    }
}
