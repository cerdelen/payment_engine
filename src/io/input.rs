/// Verifies that only 1 Argument was given to the executable.
///
/// # Returns
///
/// This function returns the first and only argument to the executable.
///
/// # Errors
///
/// This function will return an error if a wrong amount of arguments was passed to the executable.
pub fn verify_arg_count(mut args: Vec<String>) -> Result<String, String> {
    if args.len() != 2 {
        eprintln!("Incorrect Usage. Please provide only 1 Argument, a CSV File.");
        if !args.is_empty() {
            return Err(format!("Usage: {} <CSV File>", args[0]));
        } else {
            return Err(String::from("Usage: transaction_resolver <CSV File>"));
        }
    };
    // safe to pop as we know the args vector has 2 Values (executable name + CSV File name)
    Ok(args.pop().unwrap())
}

/// Opens the given File and wraps it in a csv Reader to be used to iterate through records.
///
/// # Errors
///
/// This function will return an error if there was a problem opening the file with the csv library.
pub fn get_transactions_reader(file_name: &str) -> Result<csv::Reader<std::fs::File>, String> {
    csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .has_headers(true)
        .from_path(file_name)
        .map_err(|e| format!("Failed to open CSV File '{file_name}': {e}"))
}

#[test]
fn wrong_parameter_count() {
    assert!(verify_arg_count(Vec::new()).is_err());

    let args: Vec<String> = vec!["transaction_resolver".into()];
    assert!(verify_arg_count(args).is_err());

    let args: Vec<String> = vec![
        "transaction_resolver".into(),
        "csv_file".into(),
        "additional_argument".into(),
    ];
    assert!(verify_arg_count(args).is_err());
}

#[test]
fn file_doesnt_exist() {}
