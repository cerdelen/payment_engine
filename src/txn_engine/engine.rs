use std::{collections::HashMap, fmt::Display};

use crate::txn_engine::{
    account::{AccountError, ClientAccount, ClientId},
    transaction::{
        ProcessedTransaction, TransactionInput, TransactionStatus, TransactionType, TxId,
    },
};

pub type AccountBalances = HashMap<ClientId, ClientAccount>;

#[derive(Debug, Default)]
pub struct TransactionEngine {
    /// Holds all ClientAccounts encountered
    accounts: AccountBalances,
    /// Holds all previously processed Deposits and Withdrawals
    deposits: HashMap<TxId, ProcessedTransaction>,
    // a counter for debug purposes to specify which transaction has failed
    // txn_counter: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TransactionError {
    AccountError(AccountError),
    DuplicatedTransactionId,
    AmtMissing,
    AmtPresent,
    TransactionIdNotExistent,
    TransactionNotDisputed,
    TransactionNotDisputable,
    ClientIdMismatch,
}

impl From<AccountError> for TransactionError {
    fn from(value: AccountError) -> Self {
        Self::AccountError(value)
    }
}

impl Display for TransactionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransactionError::AccountError(account_error) => {
                write!(f, "account error: {account_error}")
            }
            TransactionError::DuplicatedTransactionId => write!(f, "duplicated transaction id"),
            TransactionError::AmtMissing => write!(f, "amt missing"),
            TransactionError::AmtPresent => write!(f, "amt present"),
            TransactionError::TransactionIdNotExistent => {
                write!(f, "transaction id referred to not existent")
            }
            TransactionError::TransactionNotDisputed => write!(f, "transaction is not disputed"),
            TransactionError::TransactionNotDisputable => {
                write!(f, "transaction is not disputable")
            }
            TransactionError::ClientIdMismatch => write!(f, "client id mismatch"),
        }
    }
}

impl TransactionEngine {
    pub fn process_transaction(&mut self, tx: TransactionInput) -> Result<(), TransactionError> {
        // Check for duplicated transaction id's for deposits or withdrawals
        // disputs, resolves and chargebacks will reference previous tx_ids with the tx_id field
        if (tx.tx_type == TransactionType::Deposit || tx.tx_type == TransactionType::Withdrawal)
            && self.deposits.contains_key(&tx.tx_id)
        {
            return Err(TransactionError::DuplicatedTransactionId);
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

    fn handle_deposit(&mut self, tx: TransactionInput) -> Result<(), TransactionError> {
        if tx.amt.is_none() {
            return Err(TransactionError::AmtMissing);
        }

        if let Some(amt) = tx.amt {
            let account = self
                .accounts
                .entry(tx.client_id)
                .or_insert(ClientAccount::new(tx.client_id));
            account.deposit(amt)?;
            self.deposits.insert(
                tx.tx_id,
                ProcessedTransaction::new(
                    tx.client_id,
                    amt,
                ),
            );
        };

        Ok(())
    }

    fn handle_withdrawal(&mut self, tx: TransactionInput) -> Result<(), TransactionError> {
        if tx.amt.is_none() {
            return Err(TransactionError::AmtMissing);
        }

        if let Some(amt) = tx.amt {
            let account = self
                .accounts
                .entry(tx.client_id)
                .or_insert(ClientAccount::new(tx.client_id));
            account.withdraw(amt)?;
        }

        Ok(())
    }

    fn handle_dispute(&mut self, tx: TransactionInput) -> Result<(), TransactionError> {
        if tx.amt.is_some() {
            return Err(TransactionError::AmtPresent);
        }

        let processed_tx = self
            .deposits
            .get_mut(&tx.tx_id)
            .ok_or(TransactionError::TransactionIdNotExistent)?;

        // client id mismatch
        if tx.client_id != processed_tx.client_id {
            return Err(TransactionError::ClientIdMismatch);
        }

        // check referenced id's satus is normal
        if processed_tx.status != TransactionStatus::Normal {
            return Err(TransactionError::TransactionNotDisputable);
        }
        // client account should always exist as we have already found a valid deposit but as a fallback we create a new empty account.
        let account = self
            .accounts
            .entry(tx.client_id)
            .or_insert(ClientAccount::new(tx.client_id));

        account.dispute(processed_tx.amt)?;
        processed_tx.status = TransactionStatus::Disputed;

        Ok(())
    }

    fn handle_resolve(&mut self, tx: TransactionInput) -> Result<(), TransactionError> {
        if tx.amt.is_some() {
            return Err(TransactionError::AmtPresent);
        }

        let processed_tx = self
            .deposits
            .get_mut(&tx.tx_id)
            .ok_or(TransactionError::TransactionIdNotExistent)?;

        // client id mismatch
        if tx.client_id != processed_tx.client_id {
            return Err(TransactionError::ClientIdMismatch);
        }

        // check referenced id's satus is disputed
        if processed_tx.status != TransactionStatus::Disputed {
            return Err(TransactionError::TransactionNotDisputed);
        }
        // client account should always exist as we have already found a valid deposit but as a fallback we create a new empty account.
        let account = self
            .accounts
            .entry(tx.client_id)
            .or_insert(ClientAccount::new(tx.client_id));

        account.resolve(processed_tx.amt)?;
        processed_tx.status = TransactionStatus::Normal;

        Ok(())
    }

    fn handle_chargeback(&mut self, tx: TransactionInput) -> Result<(), TransactionError> {
        if tx.amt.is_some() {
            return Err(TransactionError::AmtPresent);
        }

        let processed_tx = self
            .deposits
            .get_mut(&tx.tx_id)
            .ok_or(TransactionError::TransactionIdNotExistent)?;

        // client id mismatch
        if tx.client_id != processed_tx.client_id {
            return Err(TransactionError::ClientIdMismatch);
        }

        // check referenced id's satus is disputed
        if processed_tx.status != TransactionStatus::Disputed {
            return Err(TransactionError::TransactionNotDisputed);
        }

        // client account should always exist as we have already found a valid deposit but as a fallback we create a new empty account.
        let account = self
            .accounts
            .entry(tx.client_id)
            .or_insert(ClientAccount::new(tx.client_id));

        account.chargeback(processed_tx.amt)?;
        processed_tx.status = TransactionStatus::ChargedBack;

        Ok(())
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
            engine.deposits.get(&1).unwrap().status,
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

        assert_eq!(engine.deposits.len(), 3);
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

        assert_eq!(engine.deposits.len(), 2);
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

        assert_eq!(engine.deposits.len(), 1);
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
        assert!(engine.deposits.is_empty());
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
        assert!(engine.deposits.is_empty());
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

        assert_eq!(engine.deposits.len(), 1);
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

        assert_eq!(engine.deposits.len(), 1);
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
        assert_eq!(engine.deposits.len(), 0);
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
        assert_eq!(engine.deposits.len(), 0);
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
        assert_eq!(engine.deposits.len(), 0);
    }
}
