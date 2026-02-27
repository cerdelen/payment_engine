use serde::Serialize;

use crate::txn_engine::amt::Amt;

pub type ClientId = u16;

#[derive(Debug, Serialize)]
pub(crate) struct ClientAccount {
    #[serde(rename = "client")]
    pub id: ClientId,
    pub available: Amt,
    pub held: Amt,
    // probably unnecessary as it can be computed when needed (held + available)
    pub total: Amt,
    pub locked: bool,
}

#[allow(unused)]
impl ClientAccount {
    pub fn new(id: ClientId) -> Self {
        Self {
            id,
            available: Amt::new(),
            held: Amt::new(),
            total: Amt::new(),
            locked: false,
        }
    }

    pub fn deposit(&mut self, amt: Amt) -> Result<(), &'static str> {
        if self.locked {
            return Err("Account is frozen");
        }

        // Check for possible overflow
        if let Some(new_total) = self.total.checked_add(amt)
            && let Some(new_available) = self.available.checked_add(amt) {
                self.total = new_total;
                self.available = new_available;
        } else {
            return Err("Deposit exceeds maximum balance");
        }

        Ok(())
    }

    pub fn withdraw(&mut self, amt: Amt) -> Result<(), &'static str> {
        if self.locked {
            return Err("Account is frozen");
        }

        if self.available < amt {
            return Err("Not enough available Funds to withdraw");
        }

        // we dont need to check for overflow since available is bigger than amt
        self.available -= amt;
        self.total -= amt;
        Ok(())
    }

    pub fn dispute(&mut self) -> Result<(), &'static str> {
        Ok(())
    }
    pub fn resolve(&mut self) -> Result<(), &'static str> {
        Ok(())
    }
    pub fn chargeback(&mut self) -> Result<(), &'static str> {
        Ok(())
    }
}
