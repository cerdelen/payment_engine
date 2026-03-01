#[cfg(test)]
mod integration_tests {
    use assert_cmd::{Command, cargo::*};
    use predicates::prelude::*;

    mod happy_path {
        use super::*;

        #[test]
        fn test_tx_id_not_order() {
            let mut cmd = Command::new(cargo_bin!("payment_engine"));

            cmd.arg("tests/input_files/tx_id_not_in_order.csv")
                .assert()
                .success()
                .stderr(predicate::str::is_empty())
                .stdout(predicate::str::contains(
                    "client,available,held,total,locked",
                ))
                .stdout(predicate::str::contains("1,3.3,0.0,3.3,false"));
        }

        #[test]
        fn test_client_id_not_order() {
            let mut cmd = Command::new(cargo_bin!("payment_engine"));

            cmd.arg("tests/input_files/client_id_not_in_order.csv")
                .assert()
                .success()
                .stderr(predicate::str::is_empty())
                .stdout(predicate::str::contains(
                    "client,available,held,total,locked",
                ))
                .stdout(predicate::str::contains("1,2.2,0.0,2.2,false"))
                .stdout(predicate::str::contains("2,1.1,0.0,1.1,false"));
        }

        #[test]
        fn test_one_valid_of_each_transaction() {
            let mut cmd = Command::new(cargo_bin!("payment_engine"));

            cmd.arg("tests/input_files/one_valid_transaction_of_every_kind.csv")
                .assert()
                .success()
                .stderr(predicate::str::is_empty())
                .stdout(predicate::str::contains(
                    "client,available,held,total,locked",
                ))
                .stdout(predicate::str::contains("5,0.0,0.0,0.0,true"))
                .stdout(predicate::str::contains("3,0.0,10.0,10.0,false"))
                .stdout(predicate::str::contains("2,0.2,0.0,0.2,false"))
                .stdout(predicate::str::contains("1,0.0,0.0,0.0,false"))
                .stdout(predicate::str::contains("4,10.0,0.0,10.0,false"));
        }

        #[test]
        fn test_empty_file() {
            let mut cmd = Command::new(cargo_bin!("payment_engine"));

            cmd.arg("tests/input_files/empty_file.csv")
                .assert()
                .success()
                .stderr(predicate::str::is_empty())
                .stdout(predicate::str::contains(
                    "client,available,held,total,locked",
                ));
        }

        #[test]
        fn test_only_headers_file() {
            let mut cmd = Command::new(cargo_bin!("payment_engine"));

            cmd.arg("tests/input_files/only_headers.csv")
                .assert()
                .success()
                .stderr(predicate::str::is_empty())
                .stdout(predicate::str::contains(
                    "client,available,held,total,locked",
                ));
        }

        #[test]
        fn test_different_valid_input_number_formats() {
            let mut cmd = Command::new(cargo_bin!("payment_engine"));

            cmd.arg("tests/input_files/different_number_formats.csv")
                .assert()
                .success()
                .stderr(predicate::str::is_empty())
                .stdout(predicate::str::contains(
                    "client,available,held,total,locked",
                ))
                .stdout(predicate::str::contains("1,2.0,0.0,2.0,false"))
                .stdout(predicate::str::contains("2,1.0,0.0,1.0,false"));
        }

        #[test]
        fn test_csv_additional_field() {
            let mut cmd = Command::new(cargo_bin!("payment_engine"));

            cmd.arg("tests/input_files/additional_csv_field.csv")
                .assert()
                .success()
                .stderr(predicate::str::contains("Error: getting record from CSV reader: CSV error: record 1 (line: 2, byte: 22): found record with 5 fields, but the previous record has 4 fields"))
                .stdout(predicate::str::contains("client,available,held,total,locked"));
        }
    }

    mod unhappy_path {
        use super::*;

        #[test]
        fn test_withdraw_too_much() {
            let mut cmd = Command::new(cargo_bin!("payment_engine"));

            // includes a withdrawal that withdraws more than available
            // includes a withdrawal that withdraws more than available but enough in held
            cmd.arg("tests/input_files/withdraw_too_much.csv")
                .assert()
                .success()
                .stderr(predicate::str::contains("Transaction failed TransactionInput { tx_type: Withdrawal, client_id: 1, tx_id: 2, amt: Some(Amt(100000)) }: account error: not enough funds available"))
                .stderr(predicate::str::contains("Transaction failed TransactionInput { tx_type: Withdrawal, client_id: 1, tx_id: 4, amt: Some(Amt(100000)) }: account error: not enough funds available"))
                .stdout(predicate::str::contains("client,available,held,total,locked"))
                .stdout(predicate::str::contains("1,1.0,20.0,21.0,false"));
        }

        #[test]
        fn test_dispute_resolve_chargeback_refer_to_non_existent_id() {
            let mut cmd = Command::new(cargo_bin!("payment_engine"));

            cmd.arg("tests/input_files/d_r_c_with_non_existent_id.csv")
                .assert()
                .success()
                .stderr(predicate::str::contains("Transaction failed TransactionInput { tx_type: Dispute, client_id: 1, tx_id: 1, amt: None }: transaction id referred to not existent"))
                .stderr(predicate::str::contains("Transaction failed TransactionInput { tx_type: Resolve, client_id: 1, tx_id: 1, amt: None }: transaction id referred to not existent"))
                .stderr(predicate::str::contains("Transaction failed TransactionInput { tx_type: Chargeback, client_id: 1, tx_id: 1, amt: None }: transaction id referred to not existent"))
                .stdout(predicate::str::contains("client,available,held,total,locked"));
        }

        #[test]
        fn test_dispute_resolve_chargeback_to_blocked_account() {
            let mut cmd = Command::new(cargo_bin!("payment_engine"));

            // includes a dispute to a blocked acc
            // includes a resolved to a blocked acc
            // includes a chargeback to a blocked acc
            cmd.arg("tests/input_files/d_r_c_to_blocked_acc.csv")
                .assert()
                .success()
                .stderr(predicate::str::contains("Transaction failed TransactionInput { tx_type: Dispute, client_id: 1, tx_id: 2, amt: None }: account error: account is locked"))
                .stderr(predicate::str::contains("Transaction failed TransactionInput { tx_type: Resolve, client_id: 1, tx_id: 3, amt: None }: account error: account is locked"))
                .stderr(predicate::str::contains("Transaction failed TransactionInput { tx_type: Chargeback, client_id: 1, tx_id: 3, amt: None }: account error: account is locked"))
                .stdout(predicate::str::contains("client,available,held,total,locked"))
                .stdout(predicate::str::contains("1,1.0,1.0,2.0,true"));
        }

        #[test]
        fn test_dispute_resolve_chargeback_refer_to_deposit_in_wrong_state() {
            let mut cmd = Command::new(cargo_bin!("payment_engine"));

            // includes a dispute to a disputed deposit
            // includes a dispute to a chargebacked deposit
            // includes a resolved to a non disputed deposit
            // includes a resolved to a chargebacked deposit
            // includes a chargeback to a non disputed deposit
            // includes a chargeback to a chargebacked deposit
            cmd.arg("tests/input_files/d_r_c_with_wrong_deposit_states.csv")
                .assert()
                .success()
                .stderr(predicate::str::contains("Transaction failed TransactionInput { tx_type: Dispute, client_id: 1, tx_id: 1, amt: None }: transaction is not disputable"))
                .stderr(predicate::str::contains("Transaction failed TransactionInput { tx_type: Resolve, client_id: 2, tx_id: 2, amt: None }: transaction is not disputed"))
                .stderr(predicate::str::contains("Transaction failed TransactionInput { tx_type: Chargeback, client_id: 2, tx_id: 2, amt: None }: transaction is not disputed"))
                .stderr(predicate::str::contains("Transaction failed TransactionInput { tx_type: Dispute, client_id: 3, tx_id: 3, amt: None }: transaction is not disputable"))
                .stderr(predicate::str::contains("Transaction failed TransactionInput { tx_type: Resolve, client_id: 3, tx_id: 3, amt: None }: transaction is not disputed"))
                .stderr(predicate::str::contains("Transaction failed TransactionInput { tx_type: Chargeback, client_id: 3, tx_id: 3, amt: None }: transaction is not disputed"))
                .stdout(predicate::str::contains("client,available,held,total,locked"))
                .stdout(predicate::str::contains("2,10.0,0.0,10.0,false"))
                .stdout(predicate::str::contains("3,0.0,0.0,0.0,true"))
                .stdout(predicate::str::contains("1,0.0,10.0,10.0,false"));
        }

        #[test]
        fn test_transaction_overflowing_internal_states() {
            let mut cmd = Command::new(cargo_bin!("payment_engine"));

            // includes a deposit to overflow available
            // includes a dispute to overflows held
            // includes a resolve to overflows available
            cmd.arg("tests/input_files/test_overflow_internal_states.csv")
                .assert()
                .success()
                .stderr(predicate::str::contains("Transaction failed TransactionInput { tx_type: Deposit, client_id: 1, tx_id: 2, amt: Some(Amt(10000)) }: account error: overflows available funds"))
                .stderr(predicate::str::contains("Transaction failed TransactionInput { tx_type: Dispute, client_id: 1, tx_id: 2, amt: None }: account error: overflows held funds"))
                .stderr(predicate::str::contains("Transaction failed TransactionInput { tx_type: Resolve, client_id: 1, tx_id: 1, amt: None }: account error: overflows available funds"))
                .stdout(predicate::str::contains("client,available,held,total,locked"))
                .stdout(predicate::str::contains("1,17014118346046923173168730371588410.5727,17014118346046923173168730371588410.5727,17014118346046923173168730371588410.5727,false"));
        }

        #[test]
        fn test_input_invalid_overflow_values() {
            let mut cmd = Command::new(cargo_bin!("payment_engine"));

            cmd.arg("tests/input_files/test_decimal_overflow_boundaries.csv")
                .assert()
                .success()
                .stderr(predicate::str::contains("Error: getting record from CSV reader: CSV deserialize error: record 2 (line: 3, byte: 75): value overflow"))
                .stderr(predicate::str::contains("Error: getting record from CSV reader: CSV deserialize error: record 3 (line: 4, byte: 128): value overflow"))
                .stderr(predicate::str::contains("Error: getting record from CSV reader: CSV deserialize error: record 5 (line: 6, byte: 237): value overflow"))
                .stderr(predicate::str::contains("Error: getting record from CSV reader: CSV deserialize error: record 6 (line: 7, byte: 293): value overflow"))
                .stdout(predicate::str::contains("client,available,held,total,locked"))
                .stdout(predicate::str::contains("1,17014118346046923173168730371588410.5727,0.0,17014118346046923173168730371588410.5727,false"));
        }

        #[test]
        fn test_duplicated_tx_ids() {
            let mut cmd = Command::new(cargo_bin!("payment_engine"));

            // File includes a withdrawal and deposits having duplicated ids already taken by
            // another deposit or withdrawal (all possible 4 combinations)
            cmd.arg("tests/input_files/duplicated_tx_id.csv")
                .assert()
                .success()
                .stderr(predicate::str::contains("Transaction failed TransactionInput { tx_type: Deposit, client_id: 1, tx_id: 1, amt: Some(Amt(10000)) }: duplicated transaction id"))
                .stderr(predicate::str::contains("Transaction failed TransactionInput { tx_type: Withdrawal, client_id: 1, tx_id: 1, amt: Some(Amt(10000)) }: duplicated transaction id"))
                .stderr(predicate::str::contains("Transaction failed TransactionInput { tx_type: Withdrawal, client_id: 1, tx_id: 2, amt: Some(Amt(10000)) }: duplicated transaction id"))
                .stderr(predicate::str::contains("Transaction failed TransactionInput { tx_type: Deposit, client_id: 1, tx_id: 2, amt: Some(Amt(10000)) }: duplicated transaction id"))
                .stdout(predicate::str::contains("client,available,held,total,locked"))
                .stdout(predicate::str::contains("1,0.0,0.0,0.0,false"));
        }

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
                .stdout(predicate::str::is_empty())
                .stderr(predicate::str::contains(format!("Error: \"Failed to open CSV File '{bogus_file_name}': No such file or directory (os error 2)\"")));
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

        #[test]
        fn test_csv_missing_field() {
            let mut cmd = Command::new(cargo_bin!("payment_engine"));

            cmd.arg("tests/input_files/missing_csv_field.csv")
                .assert()
                .success()
                .stderr(predicate::str::contains("Error: getting record from CSV reader: CSV error: record 1 (line: 2, byte: 16): found record with 3 fields, but the previous record has 4 fields"))
                .stdout(predicate::str::contains("client,available,held,total,locked"));
        }

        #[test]
        fn test_csv_invalid_field_values() {
            let mut cmd = Command::new(cargo_bin!("payment_engine"));

            cmd.arg("tests/input_files/wrong_values_for_fields.csv")
                .assert()
                .success()
                .stderr(predicate::str::contains("Error: getting record from CSV reader: CSV deserialize error: record 1 (line: 2, byte: 22): unknown variant `foobar`, expected one of `deposit`, `withdrawal`, `dispute`, `resolve`, `chargeback"))
                .stderr(predicate::str::contains("Error: getting record from CSV reader: CSV deserialize error: record 2 (line: 3, byte: 40): field 1: invalid digit found in string"))
                .stderr(predicate::str::contains("Error: getting record from CSV reader: CSV deserialize error: record 3 (line: 4, byte: 61): field 1: invalid digit found in string"))
                .stderr(predicate::str::contains("Error: getting record from CSV reader: CSV deserialize error: record 4 (line: 5, byte: 84): field 1: invalid digit found in string"))
                .stderr(predicate::str::contains("Error: getting record from CSV reader: CSV deserialize error: record 5 (line: 6, byte: 107): field 1: invalid digit found in string"))
                .stderr(predicate::str::contains("Error: getting record from CSV reader: CSV deserialize error: record 6 (line: 7, byte: 133): field 1: invalid digit found in string"))
                .stdout(predicate::str::contains("client,available,held,total,locked"));
        }
    }
}
