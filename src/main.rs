use anyhow::{self, Result};
use std::env;

mod io;
mod txn_engine;

use input::{get_transactions_reader, verify_arg_count};

use output::print_account_balances;

use crate::{
    io::{input, output},
    txn_engine::{engine::TransactionEngine, transaction::TransactionInput},
};

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    run_main(args)?;
    Ok(())
}

fn run_main(args: Vec<String>) -> Result<()> {
    let csv_file_path = verify_arg_count(args)?;
    let mut txn_reader = get_transactions_reader(&csv_file_path)?;

    let mut engine = TransactionEngine::default();

    for record_res in txn_reader.deserialize::<TransactionInput>() {
        match record_res {
            Ok(tx) => {
                if let Err(e) = engine.process_transaction(tx) {
                    eprintln!("Transaction failed: {e}");
                }
            },
            Err(e) => eprintln!("Error: getting record from CSV reader: {e}"),
        }
    }

    print_account_balances(engine.get_account_balances());

    Ok(())
}
