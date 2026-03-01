#[cfg(test)]
mod integration_tests {
    use assert_cmd::{Command, cargo::*};
    use predicates::prelude::*;

    #[test]
    fn test_simple_valid_input() {
        let mut cmd = Command::new(cargo_bin!("payment_engine"));

        cmd.arg("tests/input_files/example_input.csv")
            .assert()
            .success()
            .stderr(predicate::str::contains("Transaction failed TransactionInput { tx_type: Withdrawal, client_id: 2, tx_id: 5, amt: Some(Amt(30505)) }: account error: not enough funds available"))
            .stdout(predicate::str::contains("client,available,held,total,locked"))
            .stdout(predicate::str::contains("1,1.4445,0.0,1.4445,false"))
            .stdout(predicate::str::contains("2,2.0,0.0,2.0,false"));
    }

    #[test]
    fn test_file_not_found() {
        let mut cmd = Command::new(cargo_bin!("payment_engine"));

        let bogus_file_name = "greherhersedfwseg";

        cmd.arg(bogus_file_name)
            .assert()
            .failure()
            .stderr(predicate::str::contains(format!("Error: \"Failed to open CSV File '{bogus_file_name}': No such file or directory (os error 2)\"")));
    }

    #[test]
    fn test_empty_file() {
        let mut cmd = Command::new(cargo_bin!("payment_engine"));

        cmd.arg("tests/input_files/empty_file.csv")
            .assert()
            .success()
            .stderr(predicate::str::is_empty())
            .stdout(predicate::str::contains("client,available,held,total,locked"));
    }

    #[test]
    fn test_only_headers_file() {
        let mut cmd = Command::new(cargo_bin!("payment_engine"));

        cmd.arg("tests/input_files/only_headers.csv")
            .assert()
            .success()
            .stderr(predicate::str::is_empty())
            .stdout(predicate::str::contains("client,available,held,total,locked"));
    }

    #[test]
    fn test_invalid_negative_amt_input() {
        let mut cmd = Command::new(cargo_bin!("payment_engine"));

        cmd.arg("tests/input_files/negative_amt_input.csv")
            .assert()
            .success()
            .stderr(predicate::str::contains("Transaction failed TransactionInput { tx_type: Deposit, client_id: 1, tx_id: 1, amt: Some(Amt(-11000)) }: amt value negative (only positive values allowed)"))
            .stderr(predicate::str::contains("Transaction failed TransactionInput { tx_type: Withdrawal, client_id: 1, tx_id: 1, amt: Some(Amt(-11000)) }: amt value negative (only positive values allowed)"))
            .stderr(predicate::str::contains("Transaction failed TransactionInput { tx_type: Dispute, client_id: 1, tx_id: 1, amt: Some(Amt(-11000)) }: amt value negative (only positive values allowed)"))
            .stderr(predicate::str::contains("Transaction failed TransactionInput { tx_type: Resolve, client_id: 1, tx_id: 1, amt: Some(Amt(-11000)) }: amt value negative (only positive values allowed)"))
            .stderr(predicate::str::contains("Transaction failed TransactionInput { tx_type: Chargeback, client_id: 1, tx_id: 1, amt: Some(Amt(-11000)) }: amt value negative (only positive values allowed)"))
            .stdout(predicate::str::contains("client,available,held,total,locked"));
    }


    #[test]
    fn test_different_input_number_formats() {
        let mut cmd = Command::new(cargo_bin!("payment_engine"));

        cmd.arg("tests/input_files/different_number_formats.csv")
            .assert()
            .success()
            .stderr(predicate::str::is_empty())
            .stdout(predicate::str::contains("client,available,held,total,locked"))
            .stdout(predicate::str::contains("1,2.0,0.0,2.0,false"))
            .stdout(predicate::str::contains("2,1.0,0.0,1.0,false"));
    }

    #[test]
    fn test_ten_txn_valid_input() {
        let mut cmd = Command::new(cargo_bin!("payment_engine"));

        cmd.arg("tests/input_files/10_tx_valid.csv")
            .assert()
            .success()
            .stderr(predicate::str::contains("Transaction failed TransactionInput { tx_type: Dispute, client_id: 2, tx_id: 3, amt: None }: account error: not enough funds available"))
            .stdout(predicate::str::contains("client,available,held,total,locked"))
            .stdout(predicate::str::contains("1,1.3,0.0,1.3,false"))
            .stdout(predicate::str::contains("2,8.5,0.0,8.5,false"))
            .stdout(predicate::str::contains("3,112.0,0.0,112.0,false"));
    }

    #[test]
    fn test_ai_created_input() {
        let mut cmd = Command::new(cargo_bin!("payment_engine"));

        cmd.arg("tests/input_files/ai_input_test_file.csv")
            .assert()
            .success()
            .stderr(predicate::str::contains("Transaction failed TransactionInput { tx_type: Dispute, client_id: 1, tx_id: 1, amt: None }: account error: account is locked"))
            .stdout(predicate::str::contains("client,available,held,total,locked"))
            .stdout(predicate::str::contains("2,400.0,0.0,400.0,true"))
            .stdout(predicate::str::contains("3,0.0,0.0,0.0,true"))
            .stdout(predicate::str::contains("1,1350.0,0.0,1350.0,true"));
    }
}
