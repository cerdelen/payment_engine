use std::{
    collections::{HashMap, TryReserveError},
    fmt::Display,
};

use crate::txn_engine::{
    account::{AccountError, ClientAccount, ClientId},
    transaction::{
        ProcessedTransaction, TransactionInput, TransactionStatus, TransactionType, TxId,
    },
};

pub type AccountBalances = HashMap<ClientId, ClientAccount>;

/// Stateful transaction engine for processing transaction streams.
///
/// This struct maintains client account states (available / held funds, blocked status) and tracks
/// processed deposits for dispute/chargeback operations. It provides a single entry point
/// [`process_transaction`], for sequential transaction processing.
///
/// # Features
///
/// - **Account tracking**: Maintains 'available', 'held', 'total' and 'blocked' state per client
/// - **Possible transactions**: 'deposit', 'withdrawal', 'dispute', 'resolve' and 'chargeback'
/// - **Dispute lifecycle**: Supports 'deposit' -> 'dispute' -> 'resolve/chargeback'
/// - **Precision**: Exact 4-decimal arithmetic using scaled i128 integer
#[derive(Debug, Default)]
pub struct TransactionEngine {
    /// Holds all [`ClientAccounts`] encountered
    accounts: AccountBalances,
    /// Holds all previously processed deposits as [`ProcessedTransaction`].
    deposits: HashMap<TxId, ProcessedTransaction>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TransactionError {
    AccountError(AccountError),
    NotEnoughMemoryAvailable(TryReserveError),
    DuplicatedTransactionId,
    AmtMissing,
    AmtPresent,
    TransactionIdNotExistent,
    TransactionNotDisputed,
    TransactionNotDisputable,
    ClientIdMismatch,
    NegativeAmtValue,
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
            TransactionError::NotEnoughMemoryAvailable(try_reserve_error) => {
                write!(
                    f,
                    "error trying to reserve space for internal states: {try_reserve_error}"
                )
            }
            TransactionError::DuplicatedTransactionId => write!(f, "duplicated transaction id"),
            TransactionError::AmtMissing => write!(f, "amt missing"),
            TransactionError::AmtPresent => write!(f, "amt present"),
            TransactionError::NegativeAmtValue => {
                write!(f, "amt value negative (only positive values allowed)")
            }
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
    const BATCH_RESERVING: usize = 50;
    /// Processes a single transaction and updates account states.
    ///
    /// Handles following transaction types:
    /// - `TransactionType::Deposit`: Adds funds to available balance
    /// - `TransactionType::Withdrawal`: Deducts funds from available balance
    /// - `TransactionType::Dispute`: Decreases available balance by deposit amt and increases held
    ///   by the same amt. Only deposits are disputable.
    /// - `TransactionType::Resolve`: Returns disputed amt from held to available balance.
    /// - `TransactionType::Chargeback`: Removes disputed amt permanently from held balance.
    ///
    /// # Errors
    ///
    /// On Error returns [`TransactionError`].
    pub fn process_transaction(&mut self, tx: TransactionInput) -> Result<(), TransactionError> {
        // Check for duplicated transaction ids for deposits or withdrawals
        // disputes, resolves and chargebacks will reference previous tx_ids with the tx_id field
        if (tx.tx_type == TransactionType::Deposit || tx.tx_type == TransactionType::Withdrawal)
            && self.deposits.contains_key(&tx.tx_id)
        {
            return Err(TransactionError::DuplicatedTransactionId);
        }

        if let Some(amt) = tx.amt
            && amt.is_negative()
        {
            return Err(TransactionError::NegativeAmtValue);
        }

        // Deposits/accounts grow unbounded. Reserve proactively to prevent allocation panics.
        // Use try_reserve + error propagation.
        if self.deposits.capacity() <= self.deposits.len() {
            self.deposits
                .try_reserve(Self::BATCH_RESERVING)
                .map_err(|e| TransactionError::NotEnoughMemoryAvailable(e))?;
        }

        if self.accounts.capacity() <= self.accounts.len() {
            self.accounts
                .try_reserve(Self::BATCH_RESERVING)
                .map_err(|e| TransactionError::NotEnoughMemoryAvailable(e))?;
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

    /// Handles a Deposit. Delegates to [`ClientAccount::deposit()`] after input Validation.
    ///
    /// # Errors
    ///
    /// This function will return an error if the transaction does not specify an Amt to deposit
    /// or propagates Errors from [`ClientAccount::deposit()`].
    fn handle_deposit(&mut self, tx: TransactionInput) -> Result<(), TransactionError> {
        let amt = tx.amt.ok_or(TransactionError::AmtMissing)?;

        let account = self
            .accounts
            .entry(tx.client_id)
            .or_insert(ClientAccount::new(tx.client_id));
        account.deposit(amt)?;
        self.deposits
            .insert(tx.tx_id, ProcessedTransaction::new(tx.client_id, amt));

        Ok(())
    }

    /// Handles a Withdrawal. Delegates to [`ClientAccount::withdraw()`] after input Validation.
    ///
    /// # Errors
    ///
    /// This function will return an error if the transaction does not specify an Amt to withdraw
    /// or propagates Errors from [`ClientAccount::withdraw()`].
    fn handle_withdrawal(&mut self, tx: TransactionInput) -> Result<(), TransactionError> {
        let amt = tx.amt.ok_or(TransactionError::AmtMissing)?;

        let account = self
            .accounts
            .entry(tx.client_id)
            .or_insert(ClientAccount::new(tx.client_id));
        account.withdraw(amt)?;

        Ok(())
    }

    /// Handles a dispute. Delegates to [`ClientAccount::dispute()`] after input Validation.
    /// On success will set the [`TransactionStatus`] of the disputed Transaction to [`TransactionStatus::Disputed`].
    /// On success will reduce available balance and increase held balance by the amt of the disputed deposit.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - transaction specifies an Amt (A dispute takes the Amt from the disputed deposit)
    /// - referenced [`TxId`] does not exist
    /// - transaction [`ClientId`] does not match the [`Client`] of the disputed deposit
    /// - disputed deposit is already disputed / chargebacked
    /// - referenced transaction is a withdrawal (will be returned as TransactionIdNotExistent Error)
    /// - any Errors propagated by [`ClientAccount::dispute()`]
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

        // check referenced id's status is normal
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

    /// Handles a resolve. Delegates to [`ClientAccount::resolve()`] after input Validation.
    /// On success will set the [`TransactionStatus`] of the disputed Transaction to [`TransactionStatus::Normal`].
    /// On success will reduce held balance and increase available balance by the amt of the disputed deposit.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - transaction specifies an Amt (A resolve takes the Amt from the disputed deposit)
    /// - referenced [`TxId`] does not exist
    /// - transaction [`ClientId`] does not match the [`Client`] of the disputed deposit
    /// - disputed deposit is not disputed
    /// - referenced transaction is a withdrawal (will be returned as TransactionIdNotExistent Error)
    /// - any Errors propagated by [`ClientAccount::resolve()`]
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

        // check referenced id's status is disputed
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

    /// Handles a chargeback. Delegates to [`ClientAccount::chargeback()`] after input Validation.
    /// On success will set the [`TransactionStatus`] of the disputed Transaction to [`TransactionStatus::ChargedBack`].
    /// On success will reduce held balance by the amt of the disputed deposit.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - transaction specifies an Amt (A resolve takes the Amt from the disputed deposit)
    /// - referenced [`TxId`] does not exist
    /// - transaction [`ClientId`] does not match the [`Client`] of the disputed deposit
    /// - disputed deposit is not disputed
    /// - referenced transaction is a withdrawal (will be returned as TransactionIdNotExistent Error)
    /// - any Errors propagated by [`ClientAccount::resolve()`]
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

        // check referenced id's status is disputed
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
mod transaction_tests {
    use crate::txn_engine::{amt::Amt, transaction::TransactionStatus};

    use super::*;

    mod deposit {
        use super::*;

        #[test]
        fn test_valid_deposits() {
            let mut engine = TransactionEngine::default();

            engine
                .process_transaction(TransactionInput {
                    tx_type: TransactionType::Deposit,
                    client_id: 1,
                    tx_id: 1,
                    amt: Some(Amt::from(1)),
                })
                .unwrap();
            assert_eq!(engine.accounts.get(&1).unwrap().available, Amt::from(1));
            engine
                .process_transaction(TransactionInput {
                    tx_type: TransactionType::Deposit,
                    client_id: 1,
                    tx_id: 2,
                    amt: Some(Amt::from(2)),
                })
                .unwrap();
            assert_eq!(engine.accounts.get(&1).unwrap().available, Amt::from(3));
            engine
                .process_transaction(TransactionInput {
                    tx_type: TransactionType::Deposit,
                    client_id: 1,
                    tx_id: 3,
                    amt: Some(Amt::from(3)),
                })
                .unwrap();
            assert_eq!(engine.accounts.get(&1).unwrap().available, Amt::from(6));
            assert_eq!(engine.accounts.len(), 1);
        }

        #[test]
        fn test_valid_deposit_create_normal_transaction_status() {
            let mut engine = TransactionEngine::default();

            engine
                .process_transaction(TransactionInput {
                    tx_type: TransactionType::Deposit,
                    client_id: 1,
                    tx_id: 1,
                    amt: Some(Amt::from(1)),
                })
                .unwrap();
            assert_eq!(
                engine.deposits.get(&1).unwrap().status,
                TransactionStatus::Normal
            );
        }

        #[test]
        fn test_correct_deposit_map_len_after_valid_deposits() {
            let mut engine = TransactionEngine::default();

            engine
                .process_transaction(TransactionInput {
                    tx_type: TransactionType::Deposit,
                    client_id: 1,
                    tx_id: 1,
                    amt: Some(Amt::from(1)),
                })
                .unwrap();
            engine
                .process_transaction(TransactionInput {
                    tx_type: TransactionType::Deposit,
                    client_id: 1,
                    tx_id: 2,
                    amt: Some(Amt::from(2)),
                })
                .unwrap();
            engine
                .process_transaction(TransactionInput {
                    tx_type: TransactionType::Deposit,
                    client_id: 1,
                    tx_id: 3,
                    amt: Some(Amt::from(3)),
                })
                .unwrap();

            assert_eq!(engine.deposits.len(), 3);
        }

        #[test]
        fn test_deposit_without_amt() {
            let mut engine = TransactionEngine::default();

            assert_eq!(
                engine.process_transaction(TransactionInput {
                    tx_type: TransactionType::Withdrawal,
                    client_id: 1,
                    tx_id: 1,
                    amt: None,
                }),
                Err(TransactionError::AmtMissing)
            );

            assert!(engine.accounts.is_empty());
            assert!(engine.deposits.is_empty());
        }

        #[test]
        fn test_duplicated_tx_id_for_deposit_ignored() {
            let mut engine = TransactionEngine::default();

            engine
                .process_transaction(TransactionInput {
                    tx_type: TransactionType::Deposit,
                    client_id: 1,
                    tx_id: 1,
                    amt: Some(Amt::from(1)),
                })
                .unwrap();
            assert_eq!(
                engine.process_transaction(TransactionInput {
                    tx_type: TransactionType::Deposit,
                    client_id: 1,
                    tx_id: 1,
                    amt: Some(Amt::from(2)),
                }),
                Err(TransactionError::DuplicatedTransactionId)
            );

            assert_eq!(engine.deposits.len(), 1);
        }
    }

    mod withdrawal {
        use super::*;

        #[test]
        fn test_valid_withdrawal() {
            let mut engine = TransactionEngine::default();

            engine
                .process_transaction(TransactionInput {
                    tx_type: TransactionType::Deposit,
                    client_id: 1,
                    tx_id: 1,
                    amt: Some(Amt::from(1)),
                })
                .unwrap();
            engine
                .process_transaction(TransactionInput {
                    tx_type: TransactionType::Withdrawal,
                    client_id: 1,
                    tx_id: 2,
                    amt: Some(Amt::from(1)),
                })
                .unwrap();
            assert_eq!(engine.accounts.get(&1).unwrap().available, Amt::from(0));

            engine
                .process_transaction(TransactionInput {
                    tx_type: TransactionType::Deposit,
                    client_id: 1,
                    tx_id: 3,
                    amt: Some(Amt::from(1000)),
                })
                .unwrap();
            engine
                .process_transaction(TransactionInput {
                    tx_type: TransactionType::Withdrawal,
                    client_id: 1,
                    tx_id: 4,
                    amt: Some(Amt::from(500)),
                })
                .unwrap();
            assert_eq!(engine.accounts.get(&1).unwrap().available, Amt::from(500));
        }

        #[test]
        fn test_withdrawal_without_amt() {
            let mut engine = TransactionEngine::default();

            assert_eq!(
                engine.process_transaction(TransactionInput {
                    tx_type: TransactionType::Deposit,
                    client_id: 1,
                    tx_id: 1,
                    amt: None,
                }),
                Err(TransactionError::AmtMissing)
            );

            assert!(engine.accounts.is_empty());
            assert!(engine.deposits.is_empty());
        }

        #[test]
        fn test_duplicated_tx_id_for_withdrawal_ignored() {
            let mut engine = TransactionEngine::default();

            engine
                .process_transaction(TransactionInput {
                    tx_type: TransactionType::Deposit,
                    client_id: 1,
                    tx_id: 1,
                    amt: Some(Amt::from(1)),
                })
                .unwrap();
            assert_eq!(
                engine.process_transaction(TransactionInput {
                    tx_type: TransactionType::Withdrawal,
                    client_id: 1,
                    tx_id: 1,
                    amt: Some(Amt::from(1)),
                }),
                Err(TransactionError::DuplicatedTransactionId)
            );

            assert_eq!(engine.deposits.len(), 1);
        }
    }

    mod dispute {
        use super::*;

        #[test]
        fn test_valid_dispute() {
            let mut engine = TransactionEngine::default();

            engine
                .process_transaction(TransactionInput {
                    tx_type: TransactionType::Deposit,
                    client_id: 1,
                    tx_id: 1,
                    amt: Some(Amt::from(1)),
                })
                .unwrap();

            engine
                .process_transaction(TransactionInput {
                    tx_type: TransactionType::Dispute,
                    client_id: 1,
                    tx_id: 1,
                    amt: None,
                })
                .unwrap();

            assert_eq!(engine.deposits.len(), 1);
            assert_eq!(
                engine.deposits.get(&1).unwrap().status,
                TransactionStatus::Disputed
            );
        }

        #[test]
        fn test_dispute_with_non_existent_transaction_id() {
            let mut engine = TransactionEngine::default();

            assert_eq!(
                engine.process_transaction(TransactionInput {
                    tx_type: TransactionType::Dispute,
                    client_id: 1,
                    tx_id: 1,
                    amt: None,
                }),
                Err(TransactionError::TransactionIdNotExistent)
            );
            assert_eq!(engine.deposits.len(), 0);
        }

        #[test]
        fn test_dispute_with_client_id_mismatch() {
            let mut engine = TransactionEngine::default();

            engine
                .process_transaction(TransactionInput {
                    tx_type: TransactionType::Deposit,
                    client_id: 1,
                    tx_id: 1,
                    amt: Some(Amt::from(1)),
                })
                .unwrap();

            assert_eq!(
                engine.process_transaction(TransactionInput {
                    tx_type: TransactionType::Dispute,
                    client_id: 2,
                    tx_id: 1,
                    amt: None,
                }),
                Err(TransactionError::ClientIdMismatch)
            );
            assert_eq!(engine.deposits.len(), 1);
        }

        // Only Deposits are stored since only deposits can be disputed. Therefore the Error will
        // be TransactionIdNonExistent
        #[test]
        fn test_dispute_withdrawal() {
            let mut engine = TransactionEngine::default();

            engine
                .process_transaction(TransactionInput {
                    tx_type: TransactionType::Deposit,
                    client_id: 1,
                    tx_id: 1,
                    amt: Some(Amt::from(1)),
                })
                .unwrap();

            engine
                .process_transaction(TransactionInput {
                    tx_type: TransactionType::Withdrawal,
                    client_id: 1,
                    tx_id: 2,
                    amt: Some(Amt::from(1)),
                })
                .unwrap();

            assert_eq!(
                engine.process_transaction(TransactionInput {
                    tx_type: TransactionType::Dispute,
                    client_id: 1,
                    tx_id: 2,
                    amt: None,
                }),
                Err(TransactionError::TransactionIdNotExistent)
            );
            assert_eq!(engine.deposits.len(), 1);
        }

        #[test]
        fn test_dispute_chargeback_deposit() {
            let mut engine = TransactionEngine::default();

            engine
                .process_transaction(TransactionInput {
                    tx_type: TransactionType::Deposit,
                    client_id: 1,
                    tx_id: 1,
                    amt: Some(Amt::from(1)),
                })
                .unwrap();

            engine
                .process_transaction(TransactionInput {
                    tx_type: TransactionType::Dispute,
                    client_id: 1,
                    tx_id: 1,
                    amt: None,
                })
                .unwrap();

            engine
                .process_transaction(TransactionInput {
                    tx_type: TransactionType::Chargeback,
                    client_id: 1,
                    tx_id: 1,
                    amt: None,
                })
                .unwrap();

            assert_eq!(
                engine.process_transaction(TransactionInput {
                    tx_type: TransactionType::Dispute,
                    client_id: 1,
                    tx_id: 1,
                    amt: None,
                }),
                Err(TransactionError::TransactionNotDisputable)
            );
            assert_eq!(engine.deposits.len(), 1);
        }

        #[test]
        fn test_dispute_already_disputed_deposit() {
            let mut engine = TransactionEngine::default();

            engine
                .process_transaction(TransactionInput {
                    tx_type: TransactionType::Deposit,
                    client_id: 1,
                    tx_id: 1,
                    amt: Some(Amt::from(1)),
                })
                .unwrap();

            engine
                .process_transaction(TransactionInput {
                    tx_type: TransactionType::Dispute,
                    client_id: 1,
                    tx_id: 1,
                    amt: None,
                })
                .unwrap();

            assert_eq!(
                engine.process_transaction(TransactionInput {
                    tx_type: TransactionType::Dispute,
                    client_id: 1,
                    tx_id: 1,
                    amt: None,
                }),
                Err(TransactionError::TransactionNotDisputable)
            );
            assert_eq!(engine.deposits.len(), 1);
        }

        #[test]
        fn test_dispute_with_amt() {
            let mut engine = TransactionEngine::default();

            assert_eq!(
                engine.process_transaction(TransactionInput {
                    tx_type: TransactionType::Dispute,
                    client_id: 1,
                    tx_id: 1,
                    amt: Some(Amt::from(1)),
                }),
                Err(TransactionError::AmtPresent)
            );
            assert_eq!(engine.deposits.len(), 0);
        }
    }

    mod resolve {
        use super::*;

        #[test]
        fn test_valid_resolve() {
            let mut engine = TransactionEngine::default();

            engine
                .process_transaction(TransactionInput {
                    tx_type: TransactionType::Deposit,
                    client_id: 1,
                    tx_id: 1,
                    amt: Some(Amt::from(1)),
                })
                .unwrap();

            engine
                .process_transaction(TransactionInput {
                    tx_type: TransactionType::Dispute,
                    client_id: 1,
                    tx_id: 1,
                    amt: None,
                })
                .unwrap();

            assert_eq!(engine.deposits.len(), 1);
            assert_eq!(
                engine.deposits.get(&1).unwrap().status,
                TransactionStatus::Disputed
            );

            engine
                .process_transaction(TransactionInput {
                    tx_type: TransactionType::Resolve,
                    client_id: 1,
                    tx_id: 1,
                    amt: None,
                })
                .unwrap();

            assert_eq!(engine.deposits.len(), 1);
            assert_eq!(
                engine.deposits.get(&1).unwrap().status,
                TransactionStatus::Normal
            );
        }

        #[test]
        fn test_resolve_with_non_existent_transaction_id() {
            let mut engine = TransactionEngine::default();

            assert_eq!(
                engine.process_transaction(TransactionInput {
                    tx_type: TransactionType::Resolve,
                    client_id: 1,
                    tx_id: 1,
                    amt: None,
                }),
                Err(TransactionError::TransactionIdNotExistent)
            );
            assert_eq!(engine.deposits.len(), 0);
        }

        #[test]
        fn test_resolve_with_client_id_mismatch() {
            let mut engine = TransactionEngine::default();

            engine
                .process_transaction(TransactionInput {
                    tx_type: TransactionType::Deposit,
                    client_id: 1,
                    tx_id: 1,
                    amt: Some(Amt::from(1)),
                })
                .unwrap();

            assert_eq!(
                engine.process_transaction(TransactionInput {
                    tx_type: TransactionType::Resolve,
                    client_id: 2,
                    tx_id: 1,
                    amt: None,
                }),
                Err(TransactionError::ClientIdMismatch)
            );
            assert_eq!(engine.deposits.len(), 1);
        }

        #[test]
        fn test_resolve_with_transaction_not_disputed() {
            let mut engine = TransactionEngine::default();

            engine
                .process_transaction(TransactionInput {
                    tx_type: TransactionType::Deposit,
                    client_id: 1,
                    tx_id: 1,
                    amt: Some(Amt::from(1)),
                })
                .unwrap();

            assert_eq!(
                engine.process_transaction(TransactionInput {
                    tx_type: TransactionType::Resolve,
                    client_id: 1,
                    tx_id: 1,
                    amt: None,
                }),
                Err(TransactionError::TransactionNotDisputed)
            );
            assert_eq!(engine.deposits.len(), 1);
        }

        #[test]
        fn test_resolve_with_amt() {
            let mut engine = TransactionEngine::default();

            assert_eq!(
                engine.process_transaction(TransactionInput {
                    tx_type: TransactionType::Resolve,
                    client_id: 1,
                    tx_id: 1,
                    amt: Some(Amt::from(1)),
                }),
                Err(TransactionError::AmtPresent)
            );
            assert_eq!(engine.deposits.len(), 0);
        }

        #[test]
        fn test_resolve_withdraw() {
            let mut engine = TransactionEngine::default();

            engine
                .process_transaction(TransactionInput {
                    tx_type: TransactionType::Deposit,
                    client_id: 1,
                    tx_id: 1,
                    amt: Some(Amt::from(1)),
                })
                .unwrap();

            engine
                .process_transaction(TransactionInput {
                    tx_type: TransactionType::Withdrawal,
                    client_id: 1,
                    tx_id: 2,
                    amt: Some(Amt::from(1)),
                })
                .unwrap();

            assert_eq!(
                engine.process_transaction(TransactionInput {
                    tx_type: TransactionType::Resolve,
                    client_id: 1,
                    tx_id: 2,
                    amt: None,
                }),
                Err(TransactionError::TransactionIdNotExistent)
            );
            assert_eq!(engine.deposits.len(), 1);
        }

        #[test]
        fn test_resolve_chargebacked_transaction() {
            let mut engine = TransactionEngine::default();

            engine
                .process_transaction(TransactionInput {
                    tx_type: TransactionType::Deposit,
                    client_id: 1,
                    tx_id: 1,
                    amt: Some(Amt::from(1)),
                })
                .unwrap();

            engine
                .process_transaction(TransactionInput {
                    tx_type: TransactionType::Dispute,
                    client_id: 1,
                    tx_id: 1,
                    amt: None,
                })
                .unwrap();

            engine
                .process_transaction(TransactionInput {
                    tx_type: TransactionType::Chargeback,
                    client_id: 1,
                    tx_id: 1,
                    amt: None,
                })
                .unwrap();

            assert_eq!(
                engine.process_transaction(TransactionInput {
                    tx_type: TransactionType::Resolve,
                    client_id: 1,
                    tx_id: 1,
                    amt: None,
                }),
                Err(TransactionError::TransactionNotDisputed)
            );

            assert_eq!(engine.deposits.len(), 1);
        }
    }

    mod chargeback {
        use super::*;

        #[test]
        fn test_valid_chargeback() {
            let mut engine = TransactionEngine::default();

            engine
                .process_transaction(TransactionInput {
                    tx_type: TransactionType::Deposit,
                    client_id: 1,
                    tx_id: 1,
                    amt: Some(Amt::from(1)),
                })
                .unwrap();

            engine
                .process_transaction(TransactionInput {
                    tx_type: TransactionType::Dispute,
                    client_id: 1,
                    tx_id: 1,
                    amt: None,
                })
                .unwrap();

            assert_eq!(engine.deposits.len(), 1);
            assert_eq!(
                engine.deposits.get(&1).unwrap().status,
                TransactionStatus::Disputed
            );

            engine
                .process_transaction(TransactionInput {
                    tx_type: TransactionType::Chargeback,
                    client_id: 1,
                    tx_id: 1,
                    amt: None,
                })
                .unwrap();

            assert_eq!(engine.deposits.len(), 1);
            assert_eq!(
                engine.deposits.get(&1).unwrap().status,
                TransactionStatus::ChargedBack
            );
        }

        #[test]
        fn test_chargeback_with_non_existent_transaction_id() {
            let mut engine = TransactionEngine::default();

            assert_eq!(
                engine.process_transaction(TransactionInput {
                    tx_type: TransactionType::Chargeback,
                    client_id: 1,
                    tx_id: 1,
                    amt: None,
                }),
                Err(TransactionError::TransactionIdNotExistent)
            );
            assert_eq!(engine.deposits.len(), 0);
        }

        #[test]
        fn test_chargeback_with_client_id_mismatch() {
            let mut engine = TransactionEngine::default();

            engine
                .process_transaction(TransactionInput {
                    tx_type: TransactionType::Deposit,
                    client_id: 1,
                    tx_id: 1,
                    amt: Some(Amt::from(1)),
                })
                .unwrap();

            assert_eq!(
                engine.process_transaction(TransactionInput {
                    tx_type: TransactionType::Chargeback,
                    client_id: 2,
                    tx_id: 1,
                    amt: None,
                }),
                Err(TransactionError::ClientIdMismatch)
            );
            assert_eq!(engine.deposits.len(), 1);
        }

        #[test]
        fn test_chargeback_with_transaction_not_disputed() {
            let mut engine = TransactionEngine::default();

            engine
                .process_transaction(TransactionInput {
                    tx_type: TransactionType::Deposit,
                    client_id: 1,
                    tx_id: 1,
                    amt: Some(Amt::from(1)),
                })
                .unwrap();

            assert_eq!(
                engine.process_transaction(TransactionInput {
                    tx_type: TransactionType::Chargeback,
                    client_id: 1,
                    tx_id: 1,
                    amt: None,
                }),
                Err(TransactionError::TransactionNotDisputed)
            );
            assert_eq!(engine.deposits.len(), 1);
        }

        #[test]
        fn test_chargeback_with_amt() {
            let mut engine = TransactionEngine::default();

            assert_eq!(
                engine.process_transaction(TransactionInput {
                    tx_type: TransactionType::Chargeback,
                    client_id: 1,
                    tx_id: 1,
                    amt: Some(Amt::from(1)),
                }),
                Err(TransactionError::AmtPresent)
            );
            assert_eq!(engine.deposits.len(), 0);
        }

        #[test]
        fn test_chargeback_withdraw() {
            let mut engine = TransactionEngine::default();

            engine
                .process_transaction(TransactionInput {
                    tx_type: TransactionType::Deposit,
                    client_id: 1,
                    tx_id: 1,
                    amt: Some(Amt::from(1)),
                })
                .unwrap();

            engine
                .process_transaction(TransactionInput {
                    tx_type: TransactionType::Withdrawal,
                    client_id: 1,
                    tx_id: 2,
                    amt: Some(Amt::from(1)),
                })
                .unwrap();

            assert_eq!(
                engine.process_transaction(TransactionInput {
                    tx_type: TransactionType::Chargeback,
                    client_id: 1,
                    tx_id: 2,
                    amt: None,
                }),
                Err(TransactionError::TransactionIdNotExistent)
            );
            assert_eq!(engine.deposits.len(), 1);
        }

        #[test]
        fn test_chargeback_chargebacked_deposit() {
            let mut engine = TransactionEngine::default();

            engine
                .process_transaction(TransactionInput {
                    tx_type: TransactionType::Deposit,
                    client_id: 1,
                    tx_id: 1,
                    amt: Some(Amt::from(1)),
                })
                .unwrap();

            engine
                .process_transaction(TransactionInput {
                    tx_type: TransactionType::Dispute,
                    client_id: 1,
                    tx_id: 1,
                    amt: None,
                })
                .unwrap();

            assert_eq!(engine.deposits.len(), 1);
            assert_eq!(
                engine.deposits.get(&1).unwrap().status,
                TransactionStatus::Disputed
            );

            engine
                .process_transaction(TransactionInput {
                    tx_type: TransactionType::Chargeback,
                    client_id: 1,
                    tx_id: 1,
                    amt: None,
                })
                .unwrap();

            assert_eq!(engine.deposits.len(), 1);
            assert_eq!(
                engine.deposits.get(&1).unwrap().status,
                TransactionStatus::ChargedBack
            );

            assert_eq!(
                engine.process_transaction(TransactionInput {
                    tx_type: TransactionType::Chargeback,
                    client_id: 1,
                    tx_id: 1,
                    amt: None,
                }),
                Err(TransactionError::TransactionNotDisputed)
            );
        }
    }

    #[test]
    fn test_correct_deposit_map_len_after_valid_withdrawals() {
        let mut engine = TransactionEngine::default();

        engine
            .process_transaction(TransactionInput {
                tx_type: TransactionType::Deposit,
                client_id: 1,
                tx_id: 1,
                amt: Some(Amt::from(1)),
            })
            .unwrap();
        engine
            .process_transaction(TransactionInput {
                tx_type: TransactionType::Withdrawal,
                client_id: 1,
                tx_id: 2,
                amt: Some(Amt::from(1)),
            })
            .unwrap();
        engine
            .process_transaction(TransactionInput {
                tx_type: TransactionType::Deposit,
                client_id: 1,
                tx_id: 3,
                amt: Some(Amt::from(1000)),
            })
            .unwrap();
        engine
            .process_transaction(TransactionInput {
                tx_type: TransactionType::Withdrawal,
                client_id: 1,
                tx_id: 4,
                amt: Some(Amt::from(500)),
            })
            .unwrap();

        assert_eq!(engine.deposits.len(), 2);
    }

    #[test]
    fn test_correct_deposit_map_len_after_invalid_withdrawals() {
        let mut engine = TransactionEngine::default();

        engine
            .process_transaction(TransactionInput {
                tx_type: TransactionType::Deposit,
                client_id: 1,
                tx_id: 1,
                amt: Some(Amt::from(1)),
            })
            .unwrap();
        assert_eq!(
            engine.process_transaction(TransactionInput {
                tx_type: TransactionType::Withdrawal,
                client_id: 1,
                tx_id: 4,
                amt: Some(Amt::from(500)),
            }),
            Err(TransactionError::AccountError(
                AccountError::NotEnoughAvailable
            ))
        );

        assert_eq!(engine.deposits.len(), 1);
    }
    #[test]
    fn test_negative_value_input() {
        let mut engine = TransactionEngine::default();

        assert_eq!(
            engine.process_transaction(TransactionInput {
                tx_type: TransactionType::Deposit,
                client_id: 1,
                tx_id: 1,
                amt: Some(Amt::from(-1)),
            }),
            Err(TransactionError::NegativeAmtValue)
        );

        assert_eq!(
            engine.process_transaction(TransactionInput {
                tx_type: TransactionType::Withdrawal,
                client_id: 1,
                tx_id: 2,
                amt: Some(Amt::from(-1)),
            }),
            Err(TransactionError::NegativeAmtValue)
        );

        assert_eq!(
            engine.process_transaction(TransactionInput {
                tx_type: TransactionType::Dispute,
                client_id: 1,
                tx_id: 3,
                amt: Some(Amt::from(-1)),
            }),
            Err(TransactionError::NegativeAmtValue)
        );

        assert_eq!(
            engine.process_transaction(TransactionInput {
                tx_type: TransactionType::Resolve,
                client_id: 1,
                tx_id: 4,
                amt: Some(Amt::from(-1)),
            }),
            Err(TransactionError::NegativeAmtValue)
        );

        assert_eq!(
            engine.process_transaction(TransactionInput {
                tx_type: TransactionType::Chargeback,
                client_id: 1,
                tx_id: 5,
                amt: Some(Amt::from(-1)),
            }),
            Err(TransactionError::NegativeAmtValue)
        );
    }
}
