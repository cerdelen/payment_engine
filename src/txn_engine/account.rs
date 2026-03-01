use std::fmt::Display;

use serde::{Serialize, ser::SerializeStruct};

use crate::txn_engine::amt::Amt;

pub type ClientId = u16;

#[derive(Debug)]
pub(crate) struct ClientAccount {
    // #[serde(rename = "client")]
    pub id: ClientId,
    pub available: Amt,
    pub held: Amt,
    pub locked: bool,
}

impl Serialize for ClientAccount {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("ClientAccount", 4)?;
        state.serialize_field("client", &self.id)?;
        state.serialize_field("available", &self.available)?;
        state.serialize_field("held", &self.held)?;
        // if available + held overflows we print i128::MAX. This is a number higher than the
        // global GDP so this is a reasonable edgecase to print unprecisely
        let total = self.available.checked_add(self.held).unwrap_or(Amt::max());
        state.serialize_field("total", &total)?;
        state.serialize_field("locked", &self.locked)?;
        state.end()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum AccountError {
    AvailableOverflow,
    HeldOverflow,
    NotEnoughHeld,
    NotEnoughAvailable,
    AccountLocked,
}

impl Display for AccountError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AccountError::AvailableOverflow => write!(f, "overflows available funds"),
            AccountError::HeldOverflow => write!(f, "overflows held funds"),
            AccountError::NotEnoughHeld => write!(f, "not enough funds held"),
            AccountError::NotEnoughAvailable => write!(f, "not enough funds available"),
            AccountError::AccountLocked => write!(f, "account is locked"),
        }
    }
}

#[allow(unused)]
impl ClientAccount {
    pub fn new(id: ClientId) -> Self {
        Self {
            id,
            available: Amt::new(),
            held: Amt::new(),
            locked: false,
        }
    }

    pub fn deposit(&mut self, amt: Amt) -> Result<(), AccountError> {
        if self.locked {
            return Err(AccountError::AccountLocked);
        }

        // Check for possible overflow
        if let Some(new_available) = self.available.checked_add(amt) {
            self.available = new_available;
        } else {
            return Err(AccountError::AvailableOverflow);
        }

        Ok(())
    }

    pub fn withdraw(&mut self, amt: Amt) -> Result<(), AccountError> {
        if self.locked {
            return Err(AccountError::AccountLocked);
        }

        if self.available < amt {
            return Err(AccountError::NotEnoughAvailable);
        }

        // we do not need to check for overflow since available is bigger than amt
        self.available -= amt;
        Ok(())
    }

    pub fn dispute(&mut self, amt: Amt) -> Result<(), AccountError> {
        if self.locked {
            return Err(AccountError::AccountLocked);
        }

        if self.available < amt {
            return Err(AccountError::NotEnoughAvailable);
        }

        // Check for possible overflow
        if let Some(new_held) = self.held.checked_add(amt) {
            self.available -= amt;
            self.held = new_held;
        } else {
            return Err(AccountError::HeldOverflow);
        }

        Ok(())
    }

    pub fn resolve(&mut self, amt: Amt) -> Result<(), AccountError> {
        if self.locked {
            return Err(AccountError::AccountLocked);
        }

        if self.held < amt {
            return Err(AccountError::NotEnoughHeld);
        }

        // Check for possible overflow
        if let Some(new_available) = self.available.checked_add(amt) {
            self.available = new_available;
            self.held -= amt;
        } else {
            return Err(AccountError::AvailableOverflow);
        }

        Ok(())
    }

    pub fn chargeback(&mut self, amt: Amt) -> Result<(), AccountError> {
        if self.locked {
            return Err(AccountError::AccountLocked);
        }

        if self.held < amt {
            return Err(AccountError::NotEnoughHeld);
        }

        self.held -= amt;

        // freeze the account
        self.locked = true;

        Ok(())
    }
}

#[cfg(test)]
mod account_tests {
    use crate::txn_engine::amt::Amt;

    use super::*;

    mod deposit {
        use super::*;
        #[test]
        fn test_deposit_overflow() {
            let mut acc = ClientAccount::new(1);
            acc.available = Amt::max();

            assert_eq!(
                acc.deposit(Amt::from(1)),
                Err(AccountError::AvailableOverflow)
            );
            assert_eq!(
                acc.deposit(Amt::from(10000)),
                Err(AccountError::AvailableOverflow)
            );

            assert_eq!(acc.available, Amt::max());

            assert!(!acc.locked);
        }

        #[test]
        fn test_valid_deposits() {
            let mut acc = ClientAccount::new(1);

            acc.deposit(Amt::from(1)).unwrap();
            assert_eq!(acc.available, Amt::from(1));

            acc.deposit(Amt::from(2)).unwrap();
            assert_eq!(acc.available, Amt::from(3));

            acc.deposit(Amt::from(3)).unwrap();
            assert_eq!(acc.available, Amt::from(6));

            assert!(!acc.locked);
        }
    }

    mod withdrawal {
        use super::*;
        #[test]
        fn test_valid_withdrawal() {
            let mut acc = ClientAccount::new(1);

            // exactly to 0
            acc.available = Amt::from(1);
            acc.withdraw(Amt::from(1)).unwrap();
            assert_eq!(acc.available, Amt::from(0));

            // some is left available
            acc.available = Amt::from(1000);
            acc.withdraw(Amt::from(500)).unwrap();
            assert_eq!(acc.available, Amt::from(500));

            assert!(!acc.locked);
        }

        #[test]
        fn test_invalid_withdrawal() {
            let mut acc = ClientAccount::new(1);

            assert_eq!(
                acc.withdraw(Amt::from(1)),
                Err(AccountError::NotEnoughAvailable)
            );

            acc.available = Amt::from(1);
            assert_eq!(
                acc.withdraw(Amt::from(2)),
                Err(AccountError::NotEnoughAvailable)
            );

            assert!(!acc.locked);
        }

        #[test]
        fn test_invalid_withdrawal_with_held_funds() {
            let mut acc = ClientAccount::new(1);
            acc.held = Amt::from(1000);

            assert_eq!(
                acc.withdraw(Amt::from(1)),
                Err(AccountError::NotEnoughAvailable)
            );

            acc.available = Amt::from(1);
            assert_eq!(
                acc.withdraw(Amt::from(2)),
                Err(AccountError::NotEnoughAvailable)
            );

            assert!(!acc.locked);
        }
    }

    mod dispute {
        use super::*;

        #[test]
        fn test_dispute_not_enough_available() {
            let mut acc = ClientAccount::new(1);

            assert_eq!(
                acc.dispute(Amt::from(1)),
                Err(AccountError::NotEnoughAvailable)
            );

            acc.available = Amt::from(1);
            assert_eq!(
                acc.dispute(Amt::from(2)),
                Err(AccountError::NotEnoughAvailable)
            );

            assert!(!acc.locked);
        }

        #[test]
        fn test_dispute_overflow_held() {
            let mut acc = ClientAccount::new(1);
            acc.available = Amt::from(1);
            acc.held = Amt::max();

            assert_eq!(acc.dispute(Amt::from(1)), Err(AccountError::HeldOverflow));
            assert_eq!(acc.available, Amt::from(1));
            assert_eq!(acc.held, Amt::max());

            assert!(!acc.locked);
        }

        #[test]
        fn test_valid_dispute() {
            let mut acc = ClientAccount::new(1);
            acc.available = Amt::from(1);

            acc.dispute(Amt::from(1)).unwrap();
            assert_eq!(acc.held, Amt::from(1));
            assert_eq!(acc.available, Amt::from(0));

            acc.available = Amt::from(1000);
            acc.dispute(Amt::from(500)).unwrap();
            assert_eq!(acc.held, Amt::from(501));
            assert_eq!(acc.available, Amt::from(500));

            assert!(!acc.locked);
        }
    }

    mod resolve {
        use super::*;

        #[test]
        fn test_resolve_not_enough_held() {
            let mut acc = ClientAccount::new(1);

            assert_eq!(acc.resolve(Amt::from(1)), Err(AccountError::NotEnoughHeld));

            acc.held = Amt::from(500);

            assert_eq!(
                acc.resolve(Amt::from(1000)),
                Err(AccountError::NotEnoughHeld)
            );
            assert_eq!(acc.held, Amt::from(500));
            assert_eq!(acc.available, Amt::from(0));

            assert!(!acc.locked);
        }

        #[test]
        fn test_resolve_available_overflow() {
            let mut acc = ClientAccount::new(1);
            acc.held = Amt::from(1);
            acc.available = Amt::max();

            assert_eq!(
                acc.resolve(Amt::from(1)),
                Err(AccountError::AvailableOverflow)
            );
            assert_eq!(acc.held, Amt::from(1));
            assert_eq!(acc.available, Amt::max());

            assert!(!acc.locked);
        }

        #[test]
        fn test_valid_resolve() {
            let mut acc = ClientAccount::new(1);
            acc.held = Amt::from(1);

            acc.resolve(Amt::from(1)).unwrap();
            assert_eq!(acc.available, Amt::from(1));
            assert_eq!(acc.held, Amt::from(0));

            acc.held = Amt::from(1000);
            acc.resolve(Amt::from(500)).unwrap();
            assert_eq!(acc.available, Amt::from(501));
            assert_eq!(acc.held, Amt::from(500));

            assert!(!acc.locked);
        }
    }

    mod chargeback {
        use super::*;

        #[test]
        fn test_chargeback_not_enough_held() {
            let mut acc = ClientAccount::new(1);

            assert_eq!(
                acc.chargeback(Amt::from(1)),
                Err(AccountError::NotEnoughHeld)
            );

            acc.held = Amt::from(500);

            assert_eq!(
                acc.chargeback(Amt::from(1000)),
                Err(AccountError::NotEnoughHeld)
            );
            assert_eq!(acc.held, Amt::from(500));
            assert_eq!(acc.available, Amt::from(0));

            assert!(!acc.locked);
        }

        #[test]
        fn test_valid_chargeback() {
            {
                let mut acc = ClientAccount::new(1);

                acc.held = Amt::from(500);
                acc.chargeback(Amt::from(1)).unwrap();

                assert_eq!(acc.held, Amt::from(499));
                assert_eq!(acc.available, Amt::from(0));

                assert!(acc.locked);
            }

            {
                let mut acc = ClientAccount::new(1);

                acc.held = Amt::from(500);
                acc.chargeback(Amt::from(500)).unwrap();

                assert_eq!(acc.held, Amt::from(0));
                assert_eq!(acc.available, Amt::from(0));

                assert!(acc.locked);
            }
        }
    }

    #[test]
    fn test_blocked_acct_refuse_any_transaction() {
        let mut acc = ClientAccount::new(1);
        acc.locked = true;

        assert_eq!(acc.deposit(Amt::from(1)), Err(AccountError::AccountLocked));
        assert_eq!(acc.withdraw(Amt::from(1)), Err(AccountError::AccountLocked));
        assert_eq!(acc.dispute(Amt::from(1)), Err(AccountError::AccountLocked));
        assert_eq!(acc.resolve(Amt::from(1)), Err(AccountError::AccountLocked));
        assert_eq!(
            acc.chargeback(Amt::from(1)),
            Err(AccountError::AccountLocked)
        );

        // assert lock status did not change
        assert!(acc.locked);
    }
}
