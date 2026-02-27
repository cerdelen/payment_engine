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
        S: serde::Serializer
    {
        let mut state = serializer.serialize_struct("ClientAccount", 4)?;
        state.serialize_field("client", &self.id)?;
        state.serialize_field("available", &self.available)?;
        state.serialize_field("held", &self.held)?;
        // if available + held overflows we print i128::MAX. This is a number higher than the
        // global GDP so this is a reasonable edgecase to print unprecisely
        let total = self.available.checked_add(self.held).unwrap_or(Amt::from(i128::MAX));
        state.serialize_field("total", &total)?;
        state.serialize_field("locked", &self.locked)?;
        state.end()
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

    pub fn deposit(&mut self, amt: Amt) -> Result<(), &'static str> {
        if self.locked {
            return Err("account is frozen");
        }

        // Check for possible overflow
        if let Some(new_available) = self.available.checked_add(amt)
        {
            self.available = new_available;
        } else {
            return Err("deposit exceeds maximum balance");
        }

        Ok(())
    }

    pub fn withdraw(&mut self, amt: Amt) -> Result<(), &'static str> {
        if self.locked {
            return Err("account is frozen");
        }

        if self.available < amt {
            return Err("not enough available funds to withdraw");
        }

        // we dont need to check for overflow since available is bigger than amt
        self.available -= amt;
        Ok(())
    }

    pub fn dispute(&mut self, amt: Amt) -> Result<(), &'static str> {
        if self.locked {
            return Err("account is frozen");
        }

        if self.available < amt {
            return Err("not enough available funds to cover the disputed amt");
        }

        // Check for possible overflow
        if let Some(new_held) = self.held.checked_add(amt) {
            self.available -= amt;
            self.held = new_held;
        } else {
            return Err("dispute overflows held amt");
        }

        Ok(())
    }
    pub fn resolve(&mut self, amt: Amt) -> Result<(), &'static str> {
        if self.locked {
            return Err("account is frozen");
        }

        if self.held < amt {
            // this error should not be possible
            return Err("not enough funds held to resolve");
        }

        // Check for possible overflow
        if let Some(new_available) = self.available.checked_add(amt) {
            self.available = new_available;
            self.held -= amt;
        } else {
            return Err("resolve overflows available amt");
        }

        Ok(())
    }
    pub fn chargeback(&mut self, amt: Amt) -> Result<(), &'static str> {
        if self.locked {
            return Err("account is frozen");
        }

        if self.held < amt {
            // this error should not be possible
            return Err("not enough funds held to chargeback");
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

    // TODO
    // test deposit to frozen acc

    #[test]
    fn test_valid_deposits() {
        let mut acc = ClientAccount::new(1);

        acc.deposit(Amt::from(1)).unwrap();
        assert_eq!(acc.available, Amt::from(1));

        acc.deposit(Amt::from(2)).unwrap();
        assert_eq!(acc.available, Amt::from(3));

        acc.deposit(Amt::from(3)).unwrap();
        assert_eq!(acc.available, Amt::from(6));
    }

    #[test]
    fn test_valid_withdrawal() {
        let mut acc = ClientAccount::new(1);

        acc.deposit(Amt::from(1)).unwrap();
        acc.withdraw(Amt::from(1)).unwrap();
        assert_eq!(acc.available, Amt::from(0));

        acc.deposit(Amt::from(1000)).unwrap();
        acc.withdraw(Amt::from(500)).unwrap();
        assert_eq!(acc.available, Amt::from(500));
    }

    #[test]
    fn test_invalid_withdrawal() {
        let mut acc = ClientAccount::new(1);

        assert!(acc.withdraw(Amt::from(1)).is_err());

        acc.deposit(Amt::from(1)).unwrap();
        assert!(acc.withdraw(Amt::from(2)).is_err());
    }
}
