use std::collections::HashMap;

use crate::txn_engine::{
    account::{ClientAccount, ClientId},
    transaction::{ProcessedTransaction, TransactionInput, TransactionType, TxId},
};

pub type AccountBalances = HashMap<ClientId, ClientAccount>;

#[derive(Debug, Default)]
pub struct TransactionEngine {
    accounts: AccountBalances,
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
        todo!()
    }

    fn handle_resolve(&mut self, tx: TransactionInput) {
        if tx.amt.is_some() {
            eprintln!("Error: Transaction {:?}: dispute has an amount", tx);
            return;
        }
        todo!()
    }

    fn handle_chargeback(&mut self, tx: TransactionInput) {
        if tx.amt.is_some() {
            eprintln!("Error: Transaction {:?}: dispute has an amount", tx);
            return;
        }
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use crate::txn_engine::{amt::Amt, transaction::TransactionStatus};

    use super::*;

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
