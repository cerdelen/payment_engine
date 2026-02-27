use std::collections::HashMap;

use crate::txn_engine::{
    account::{ClientAccount, ClientId},
    transaction::{ProcessedTransaction, TransactionInput, TransactionStatus, TransactionType, TxId},
};

pub type AccountBalances = HashMap<ClientId, ClientAccount>;

#[derive(Debug, Default)]
pub struct TransactionEngine {
    /// Holds all ClientAccounts encountered
    accounts: AccountBalances,
    /// Holds all previously processed Deposits and Withdrawals
    transactions: HashMap<TxId, ProcessedTransaction>,
}

impl TransactionEngine {
    pub fn process_transaction(&mut self, tx: TransactionInput) {
        // Check for duplicated transaction id's for deposits or withdrawals
        // disputs, resolves and chargebacks will reference previous tx_ids with the tx_id field
        if (tx.tx_type == TransactionType::Deposit || tx.tx_type == TransactionType::Withdrawal)
            && self.transactions.contains_key(&tx.tx_id)
        {
            eprintln!("Error: Duplicated transaction id: {}", tx.client_id);
            return;
        }
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
        &self.accounts
    }

    fn handle_deposit(&mut self, tx: TransactionInput) {
        let res = if let Some(amt) = tx.amt {
            let account = self
                .accounts
                .entry(tx.client_id)
                .or_insert(ClientAccount::new(tx.client_id));
            account.deposit(amt).map(|_| {
                self.transactions.insert(
                    tx.tx_id,
                    ProcessedTransaction::new(
                        tx.tx_type == TransactionType::Deposit,
                        tx.client_id,
                        amt,
                    ),
                );
            })
        } else {
            Err("deposit transaction is missing an amount")
        };

        if let Err(e) = res {
            eprintln!("Error: Transaction {:?}: {e}", tx);
        }
    }

    fn handle_withdrawal(&mut self, tx: TransactionInput) {
        let res = if let Some(amt) = tx.amt {
            let account = self
                .accounts
                .entry(tx.client_id)
                .or_insert(ClientAccount::new(tx.client_id));
            account.withdraw(amt).map(|_| {
                self.transactions.insert(
                    tx.tx_id,
                    ProcessedTransaction::new(
                        tx.tx_type == TransactionType::Deposit,
                        tx.client_id,
                        amt,
                    ),
                );
            })
        } else {
            Err("withdrawal transaction is missing an amount")
        };

        if let Err(e) = res {
            eprintln!("Error: Transaction {:?}: {e}", tx);
        }
    }

    fn handle_dispute(&mut self, tx: TransactionInput) {
        if tx.amt.is_some() {
            eprintln!("Error: Transaction {:?}: dispute has an amount", tx);
            return;
        }
        let res = if let Some(processed_tx) = self.transactions.get_mut(&tx.tx_id) {
            // client id mismatch
            if tx.client_id != processed_tx.client_id {
                eprintln!("Error: Transaction {:?}: dispute referred client_id does not matched the client id of referred transaction", tx);
                return ;
            }
            // check referenced id was a deposit
            if !processed_tx.is_deposit {
                eprintln!("Error: Transaction {:?}: referenced transaction is not a deposit", tx);
                return ;
            }
            // check referenced id's satus is normal
            if processed_tx.status != TransactionStatus::Normal {
                eprintln!("Error: Transaction {:?}: referenced transaction is not disputed", tx);
                return ;
            }
            // client account should always exist as we have already found a valid deposit but as a fallback we create a new empty account.
            let account = self.accounts.entry(tx.client_id).or_insert(ClientAccount::new(tx.client_id));
            account.dispute(processed_tx.amt).map(|_| {
                processed_tx.status = TransactionStatus::Disputed;
            })
        } else {
            Err("transaction id reference in dispute does not exist")
        };

        if let Err(e) = res {
            eprintln!("Error: Transaction {:?}: {e}", tx);
        }
    }

    fn handle_resolve(&mut self, tx: TransactionInput) {
        if tx.amt.is_some() {
            eprintln!("Error: Transaction {:?}: resolve has an amount", tx);
            return;
        }

        let res = if let Some(processed_tx) = self.transactions.get_mut(&tx.tx_id) {
            // client id mismatch
            if tx.client_id != processed_tx.client_id {
                eprintln!("Error: Transaction {:?}: resolve referred client_id does not matched the client id of referred transaction", tx);
                return ;
            }
            // check referenced id was a deposit
            if !processed_tx.is_deposit {
                eprintln!("Error: Transaction {:?}: referenced transaction is not a deposit", tx);
                return ;
            }
            // check referenced id's satus is disputed
            if processed_tx.status != TransactionStatus::Disputed {
                eprintln!("Error: Transaction {:?}: referenced transaction is not disputed", tx);
                return ;
            }
            // client account should always exist as we have already found a valid deposit but as a fallback we create a new empty account.
            let account = self.accounts.entry(tx.client_id).or_insert(ClientAccount::new(tx.client_id));
            account.resolve(processed_tx.amt).map(|_| {
                processed_tx.status = TransactionStatus::Normal;
            })
        } else {
            Err("transaction id reference in resolve does not exist")
        };

        if let Err(e) = res {
            eprintln!("Error: Transaction {:?}: {e}", tx);
        }
    }

    fn handle_chargeback(&mut self, tx: TransactionInput) {
        if tx.amt.is_some() {
            eprintln!("Error: Transaction {:?}: chargeback has an amount", tx);
            return;
        }

        let res = if let Some(processed_tx) = self.transactions.get_mut(&tx.tx_id) {
            // client id mismatch
            if tx.client_id != processed_tx.client_id {
                eprintln!("Error: Transaction {:?}: chargeback referred client_id does not matched the client id of referred transaction", tx);
                return ;
            }
            // check referenced id was a deposit
            if !processed_tx.is_deposit {
                eprintln!("Error: Transaction {:?}: referenced transaction is not a deposit", tx);
                return ;
            }
            // check referenced id's satus is disputed
            if processed_tx.status != TransactionStatus::Disputed {
                eprintln!("Error: Transaction {:?}: referenced transaction is not disputed", tx);
                return ;
            }
            // client account should always exist as we have already found a valid deposit but as a fallback we create a new empty account.
            let account = self.accounts.entry(tx.client_id).or_insert(ClientAccount::new(tx.client_id));
            account.chargeback(processed_tx.amt).map(|_| {
                processed_tx.status = TransactionStatus::ChargedBack;
            })
        } else {
            Err("transaction id reference in chargeback does not exist")
        };

        if let Err(e) = res {
            eprintln!("Error: Transaction {:?}: {e}", tx);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::txn_engine::{amt::Amt, transaction::TransactionStatus};

    use super::*;
    // TODO
    // dispute valid deposit (works)
    // dispute client_id mismatch
    // dispute disputed transaction (does not work)
    // dispute chargebacked transaction (does not work)
    // dispute withrdaw (does not work)
    // resolve disputed deposit (works)
    // resolve undisputed deposit (does not work)
    // resolve withdraw (cannot be dispted -> does not work)
    // resolve chargebacked transaction (does not work)
    // resolve client_id mismatch
    // chargeback client_id mismatch
    // chargeback withrdaw (does not work)
    // chargeback undisputed transaction (does not work)
    // chargeback chargebacked transaction (does not work)

    // TODO
    // test deposit to frozen acc
    #[test]
    fn test_valid_deposits() {
        let mut engine = TransactionEngine::default();

        engine.process_transaction(TransactionInput {
            tx_type: TransactionType::Deposit,
            client_id: 1,
            tx_id: 1,
            amt: Some(Amt::from(1)),
        });
        assert_eq!(engine.accounts.get(&1).unwrap().available, Amt::from(1));
        engine.process_transaction(TransactionInput {
            tx_type: TransactionType::Deposit,
            client_id: 1,
            tx_id: 2,
            amt: Some(Amt::from(2)),
        });
        assert_eq!(engine.accounts.get(&1).unwrap().available, Amt::from(3));
        engine.process_transaction(TransactionInput {
            tx_type: TransactionType::Deposit,
            client_id: 1,
            tx_id: 3,
            amt: Some(Amt::from(3)),
        });
        assert_eq!(engine.accounts.get(&1).unwrap().available, Amt::from(6));
        assert_eq!(engine.accounts.len(), 1);
    }

    #[test]
    fn test_valid_deposit_create_normal_transaction_status() {
        let mut engine = TransactionEngine::default();

        engine.process_transaction(TransactionInput {
            tx_type: TransactionType::Deposit,
            client_id: 1,
            tx_id: 1,
            amt: Some(Amt::from(1)),
        });
        assert_eq!(
            engine.transactions.get(&1).unwrap().status,
            TransactionStatus::Normal
        );
        // assert_eq!(engine.accounts.get(&1).unwrap().available, Amt::from(6));
        // assert_eq!(engine.accounts.len(), 1);
    }

    #[test]
    fn test_correct_transactions_len_after_valid_deposits() {
        let mut engine = TransactionEngine::default();

        engine.process_transaction(TransactionInput {
            tx_type: TransactionType::Deposit,
            client_id: 1,
            tx_id: 1,
            amt: Some(Amt::from(1)),
        });
        engine.process_transaction(TransactionInput {
            tx_type: TransactionType::Deposit,
            client_id: 1,
            tx_id: 2,
            amt: Some(Amt::from(2)),
        });
        engine.process_transaction(TransactionInput {
            tx_type: TransactionType::Deposit,
            client_id: 1,
            tx_id: 3,
            amt: Some(Amt::from(3)),
        });

        assert_eq!(engine.transactions.len(), 3);
    }

    #[test]
    fn test_correct_transactions_len_after_valid_withdrawals() {
        let mut engine = TransactionEngine::default();

        engine.process_transaction(TransactionInput {
            tx_type: TransactionType::Deposit,
            client_id: 1,
            tx_id: 1,
            amt: Some(Amt::from(1)),
        });
        engine.process_transaction(TransactionInput {
            tx_type: TransactionType::Withdrawal,
            client_id: 1,
            tx_id: 2,
            amt: Some(Amt::from(1)),
        });
        engine.process_transaction(TransactionInput {
            tx_type: TransactionType::Deposit,
            client_id: 1,
            tx_id: 3,
            amt: Some(Amt::from(1000)),
        });
        engine.process_transaction(TransactionInput {
            tx_type: TransactionType::Withdrawal,
            client_id: 1,
            tx_id: 4,
            amt: Some(Amt::from(500)),
        });

        assert_eq!(engine.transactions.len(), 4);
    }

    #[test]
    fn test_correct_transactions_len_after_invalid_withdrawals() {
        let mut engine = TransactionEngine::default();

        engine.process_transaction(TransactionInput {
            tx_type: TransactionType::Deposit,
            client_id: 1,
            tx_id: 1,
            amt: Some(Amt::from(1)),
        });
        engine.process_transaction(TransactionInput {
            tx_type: TransactionType::Withdrawal,
            client_id: 1,
            tx_id: 4,
            amt: Some(Amt::from(500)),
        });

        assert_eq!(engine.transactions.len(), 1);
    }

    #[test]
    fn test_valid_withdrawal() {
        let mut engine = TransactionEngine::default();

        engine.process_transaction(TransactionInput {
            tx_type: TransactionType::Deposit,
            client_id: 1,
            tx_id: 1,
            amt: Some(Amt::from(1)),
        });
        engine.process_transaction(TransactionInput {
            tx_type: TransactionType::Withdrawal,
            client_id: 1,
            tx_id: 2,
            amt: Some(Amt::from(1)),
        });
        assert_eq!(engine.accounts.get(&1).unwrap().available, Amt::from(0));

        engine.process_transaction(TransactionInput {
            tx_type: TransactionType::Deposit,
            client_id: 1,
            tx_id: 3,
            amt: Some(Amt::from(1000)),
        });
        engine.process_transaction(TransactionInput {
            tx_type: TransactionType::Withdrawal,
            client_id: 1,
            tx_id: 4,
            amt: Some(Amt::from(500)),
        });
        assert_eq!(engine.accounts.get(&1).unwrap().available, Amt::from(500));
    }

    #[test]
    fn test_withdrawal_without_amt() {
        let mut engine = TransactionEngine::default();

        engine.process_transaction(TransactionInput {
            tx_type: TransactionType::Deposit,
            client_id: 1,
            tx_id: 1,
            amt: None,
        });

        assert!(engine.accounts.is_empty());
        assert!(engine.transactions.is_empty());
    }

    #[test]
    fn test_deposit_without_amt() {
        let mut engine = TransactionEngine::default();

        engine.process_transaction(TransactionInput {
            tx_type: TransactionType::Withdrawal,
            client_id: 1,
            tx_id: 1,
            amt: None,
        });

        assert!(engine.accounts.is_empty());
        assert!(engine.transactions.is_empty());
    }

    #[test]
    fn test_duplicated_tx_id_for_deposit_ignored() {
        let mut engine = TransactionEngine::default();

        engine.process_transaction(TransactionInput {
            tx_type: TransactionType::Deposit,
            client_id: 1,
            tx_id: 1,
            amt: Some(Amt::from(1)),
        });
        engine.process_transaction(TransactionInput {
            tx_type: TransactionType::Deposit,
            client_id: 1,
            tx_id: 1,
            amt: Some(Amt::from(2)),
        });

        assert_eq!(engine.transactions.len(), 1);
    }

    #[test]
    fn test_duplicated_tx_id_for_withdrawal_ignored() {
        let mut engine = TransactionEngine::default();

        engine.process_transaction(TransactionInput {
            tx_type: TransactionType::Deposit,
            client_id: 1,
            tx_id: 1,
            amt: Some(Amt::from(1)),
        });
        engine.process_transaction(TransactionInput {
            tx_type: TransactionType::Withdrawal,
            client_id: 1,
            tx_id: 1,
            amt: Some(Amt::from(1)),
        });

        assert_eq!(engine.transactions.len(), 1);
    }

    #[test]
    fn test_chargeback_with_amt() {
        let mut engine = TransactionEngine::default();

        engine.process_transaction(TransactionInput {
            tx_type: TransactionType::Chargeback,
            client_id: 1,
            tx_id: 1,
            amt: Some(Amt::from(1)),
        });
        assert_eq!(engine.transactions.len(), 0);
    }

    #[test]
    fn test_resolve_with_amt() {
        let mut engine = TransactionEngine::default();

        engine.process_transaction(TransactionInput {
            tx_type: TransactionType::Resolve,
            client_id: 1,
            tx_id: 1,
            amt: Some(Amt::from(1)),
        });
        assert_eq!(engine.transactions.len(), 0);
    }

    #[test]
    fn test_dispute_with_amt() {
        let mut engine = TransactionEngine::default();

        engine.process_transaction(TransactionInput {
            tx_type: TransactionType::Dispute,
            client_id: 1,
            tx_id: 1,
            amt: Some(Amt::from(1)),
        });
        assert_eq!(engine.transactions.len(), 0);
    }
}
