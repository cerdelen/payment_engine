use serde::Serialize;

use crate::txn_engine::amt::Amt;

pub type ClientId = u16;

#[derive(Debug, Serialize)]
pub(crate) struct ClientAccountBalance {
    #[serde(rename = "client")]
    pub id: ClientId,
    pub available: Amt,
    pub held: Amt,
    // probably unnecessary as it can be computed when needed (held + available)
    pub total: Amt,
    pub locked: bool,
}

#[allow(unused)]
impl ClientAccountBalance {
    pub fn new(id: ClientId) -> Self {
        Self {
            id,
            available: Amt::new(),
            held: Amt::new(),
            total: Amt::new(),
            locked: false,
        }
    }
}
