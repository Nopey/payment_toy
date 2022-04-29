// I (ab)use the underscore as a placeholder for the decimal point in this file
#![allow(clippy::inconsistent_digit_grouping)]
use rand::{prelude::SliceRandom, SeedableRng};

use super::{transaction::Action, *};

#[test]
fn process_tx_skips_dup_deposits() {
    let mut tx_history = tx_history::TxHistory::default();
    let deposit_amount = Money::from_i128(123_0000);
    let client = 725;
    let tx_id = 101;
    let tx = Transaction::new(Action::new_deposit(deposit_amount), client, tx_id);
    let mut account = Account::new(client);
    assert_eq!(Ok(()), account.process_transaction(&tx, &mut tx_history));
    for _ in 0..10 {
        assert_eq!(Err(Error::DuplicateTransaction(tx_id)), account.process_transaction(&tx, &mut tx_history));
    }
    assert!(account.available_funds == deposit_amount);
    assert!(account.held_funds == Money::ZERO);
}

#[test]
fn process_tx_allows_tx_in_locked_accounts() {
    let mut tx_history = tx_history::TxHistory::default();
    let deposit_amount = Money::from_i128(123000_0000);
    let withdrawal_amount = Money::from_i128(123_0000);
    let client = 725;
    let deposit_id: TxId = 101;
    let withdrawal_id: TxId = 102;
    let mut account = Account::new(client);

    // Lock the account
    account.locked = true;

    // process one deposit to put a balance in the account
    let deposit = Transaction::new(Action::new_deposit(deposit_amount), client, deposit_id);
    assert_eq!(Ok(()), account.process_transaction(&deposit, &mut tx_history));

    // process a withdrawal
    let withdrawal = Transaction::new(
        Action::new_withdrawal(withdrawal_amount),
        client,
        withdrawal_id,
    );
    assert_eq!(Ok(()), account.process_transaction(&withdrawal, &mut tx_history));

    assert_eq!(account.available_funds, (deposit_amount - withdrawal_amount));
    assert!(account.held_funds == Money::ZERO);
}

#[test]
fn process_tx_skips_dup_withdrawals() {
    let mut tx_history = tx_history::TxHistory::default();
    let deposit_amount = Money::from_i128(123000_0000);
    let withdrawal_amount = Money::from_i128(123_0000);
    let client = 725;
    let deposit_id: TxId = 101;
    let withdrawal_id: TxId = 102;
    let mut account = Account::new(client);
    
    // process one deposit to put a balance in the account
    let deposit = Transaction::new(Action::new_deposit(deposit_amount), client, deposit_id);
    assert_eq!(Ok(()), account.process_transaction(&deposit, &mut tx_history));

    // process ten of the same withdrawal
    let withdrawal = Transaction::new(
        Action::new_withdrawal(withdrawal_amount),
        client,
        withdrawal_id,
    );
    assert_eq!(Ok(()), account.process_transaction(&withdrawal, &mut tx_history));
    for _ in 0..10 {
        assert_eq!(Err(Error::DuplicateTransaction(withdrawal_id)), account.process_transaction(&withdrawal, &mut tx_history));
    }
    assert_eq!(account.available_funds, (deposit_amount - withdrawal_amount));
    assert!(account.held_funds == Money::ZERO);
}

fn parse_test_data(data: &[(&'static str, &'static str); 4]) -> Result<Transaction, csv::Error> {
    let mut header = csv::StringRecord::new();
    header.extend(data.iter().map(|d| d.0));
    let mut record = csv::StringRecord::new();
    record.extend(data.iter().map(|d| d.1));

    record.deserialize(Some(&header))
}

#[test]
fn csv_fields_order_doesnt_matter() {
    let mut data = [
        ("client", "100"),
        ("tx", "100"),
        ("type", "deposit"),
        ("amount", "100"),
    ];
    let baseline = parse_test_data(&data).unwrap();

    let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(1234567890);
    for _ in 0..32 {
        data.shuffle(&mut rng);
        let shuffled = parse_test_data(&data).unwrap();
        assert_eq!(baseline, shuffled);
    }
}

#[test]
fn csv_with_negative_amounts_rejected() {
    let data = [
        ("amount", "-100.0000"),
        ("tx", "100"),
        ("type", "deposit"),
        ("client", "615"),
    ];
    assert!(parse_test_data(&data).is_err());
}

#[test]
fn deposits_without_amount_rejected() {
    let data = [
        ("amount", ""),
        ("tx", "100"),
        ("type", "deposit"),
        ("client", "615"),
    ];
    assert!(parse_test_data(&data).is_err());
}

#[test]
fn withdrawals_without_amount_rejected() {
    let data = [
        ("amount", ""),
        ("tx", "100"),
        ("type", "withdrawal"),
        ("client", "615"),
    ];
    assert!(parse_test_data(&data).is_err());
}

#[test]
fn dispute_without_amount_accepted() {
    let data = [
        ("amount", ""),
        ("tx", "100"),
        ("type", "dispute"),
        ("client", "615"),
    ];
    assert!(parse_test_data(&data).is_ok());
}

#[test]
fn dispute_with_amount_rejected() {
    let data = [
        ("amount", "100.0"),
        ("tx", "100"),
        ("type", "dispute"),
        ("client", "615"),
    ];
    assert!(parse_test_data(&data).is_err());
}

#[test]
fn resolve_with_amount_rejected() {
    let data = [
        ("amount", "100.0"),
        ("tx", "100"),
        ("type", "resolve"),
        ("client", "615"),
    ];
    assert!(parse_test_data(&data).is_err());
}

#[test]
fn chargeback_with_amount_rejected() {
    let data = [
        ("amount", "100.0"),
        ("tx", "100"),
        ("type", "chargeback"),
        ("client", "615"),
    ];
    assert!(parse_test_data(&data).is_err());
}

#[test]
fn deposit_with_amount_accepted() {
    let data = [
        ("amount", "100.0"),
        ("tx", "100"),
        ("type", "deposit"),
        ("client", "615"),
    ];
    assert!(parse_test_data(&data).is_ok());
}

#[test]
fn withdrawal_with_amount_accepted() {
    let data = [
        ("amount", "100.0"),
        ("tx", "100"),
        ("type", "withdrawal"),
        ("client", "615"),
    ];
    assert!(parse_test_data(&data).is_ok());
}

#[test]
fn account_total_simple_addition() {
    let client = 266;
    let mut account = Account::new(client);
    account.available_funds = Money::from_i128(120_0000);
    account.held_funds = Money::from_i128(3_4567);
    assert!(account.total() == Money::from_i128(123_4567))
}

#[test]
fn account_total_negative_available() {
    let client = 266;
    let mut account = Account::new(client);
    account.available_funds = Money::from_i128(120_0000);
    account.available_funds.0.set_sign_negative(true);
    account.held_funds = Money::from_i128(360_0000);
    assert!(account.total() == Money::from_i128(240_0000))
}
