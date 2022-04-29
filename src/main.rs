use account::{Account, Client, Transaction, TxHistory};
use std::collections::HashMap;
use std::fs::File;
use std::path::PathBuf;

mod account;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // commandline interface
    let mut args = std::env::args_os().skip(1);
    let path = if let Some(path) = args.next() {
        PathBuf::from(path)
    } else {
        return Err("Too few arguments! Expected one argument, the input CSV file.".into());
    };
    if args.next().is_some() {
        return Err("Too many arguments! Expected one argument, the input CSV file.".into());
    }
    let file = File::open(&path)?;

    // process all transactions
    let mut csv_in = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .trim(csv::Trim::All)
        .from_reader(file);
    let mut accounts = HashMap::<Client, Account>::new();
    let mut tx_history = TxHistory::default();
    for tx in csv_in.deserialize() {
        let tx: Transaction = tx?;
        let client = tx.client();
        let account = accounts
            .entry(client)
            .or_insert_with(|| Account::new(client));
        // ignore errors from process_transaction
        account.process_transaction(&tx, &mut tx_history).ok();
    }

    // generate report
    let stdout = std::io::stdout();
    let stdout = stdout.lock();
    let mut csv_out = csv::WriterBuilder::new()
        .has_headers(true)
        .from_writer(stdout);
    for (_client, account) in accounts.into_iter() {
        csv_out.serialize(account)?;
    }

    Ok(())
}
