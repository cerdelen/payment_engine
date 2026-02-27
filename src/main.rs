use anyhow::{self, Result};
use std::{collections::HashMap, env};

mod input;
mod output;
mod txn;
mod types;

use input::{get_transactions_reader, verify_arg_count};
use types::*;

use output::print_account_balances;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    run_main(args)?;
    Ok(())
}

fn run_main(args: Vec<String>) -> Result<()> {
    let csv_file_path = verify_arg_count(args)?;
    let mut txn_reader = get_transactions_reader(&csv_file_path)?;

    let mut balances = AccountBalances::new();
    let placeholder = HashMap::new();

    for record_res in txn_reader.deserialize::<Transaction>() {
        if let Ok(tx) = record_res {
            txn::process_transaction(tx, &mut balances, &placeholder);
        } else {
            // log Error
        }
    }

    balances.insert(
        1,
        types::ClientAccountBalance {
            id: 1,
            available: Amt::from(12030),
            held: Amt::new(),
            total: Amt::from(12030),
            locked: false,
        },
    );

    balances.insert(
        2,
        types::ClientAccountBalance {
            id: 2,
            available: Amt::new(),
            held: Amt::new(),
            total: Amt::new(),
            locked: true,
        },
    );

    print_account_balances(&balances);

    Ok(())
}
