use std::io;

use crate::txn_engine::engine::AccountBalances;

pub fn print_account_balances(balances: &AccountBalances) {
    let mut writer = csv::Writer::from_writer(io::stdout());

    if balances.is_empty() {
        if let Err(e) = writer.write_record(["client", "available", "held", "total", "locked"]) {
            eprintln!("Error: writing to stdout: {e}");
        }
        return;
    }

    for balance in balances.values() {
        if let Err(e) = writer.serialize(balance) {
            eprintln!("Error: writing serialized account balance to stdout: {e}");
        }
    }
}
