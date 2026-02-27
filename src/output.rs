use std::io;

use crate::types::AccountBalances;

pub fn print_account_balances(balances: &AccountBalances) {
    let mut writer = csv::Writer::from_writer(io::stdout());

    for balance in balances.values() {
        // TODO
        // how to handle output error
        let _ = writer.serialize(balance);
    }
}
