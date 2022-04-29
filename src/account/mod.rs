mod transaction;
mod tx_history;

use derive_more::{Add, AddAssign, Display, Sub, SubAssign};
use serde::{ser::SerializeStruct, Deserialize, Serialize};
pub use transaction::{Id as TxId, Transaction};
pub use tx_history::TxHistory;

pub type Client = u16;
//pub type Money = rust_decimal::Decimal;
//pub type Money = f64;
#[derive(
    Default,
    Clone,
    Copy,
    Display,
    Debug,
    Add,
    AddAssign,
    Sub,
    SubAssign,
    PartialEq,
    PartialOrd,
    Serialize,
    Deserialize,
)]
#[serde(transparent)]
pub struct Money(#[serde(with = "rust_decimal::serde::str")] rust_decimal::Decimal);

impl Money {
    pub const ZERO: Money = Money(rust_decimal::Decimal::ZERO);
}

pub struct Account {
    client: Client,
    available_funds: Money,
    held_funds: Money,
    /// are the funds frozen?
    locked: bool,
}

impl Account {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            available_funds: Money::ZERO,
            held_funds: Money::ZERO,
            locked: false,
        }
    }
    pub fn total(&self) -> Money {
        self.available_funds + self.held_funds
    }
    pub fn process_transaction(&mut self, tx: &Transaction, tx_history: &mut TxHistory) {
        use transaction::Action::*;
        match tx.action() {
            Deposit { amount } => {
                if tx_history
                    .record_transaction(tx.id(), amount, tx_history::CompletedTxKind::Deposit)
                    .is_err()
                {
                    // silently ignore duplicate transactions
                    return;
                };
                self.available_funds += amount;
            }
            Withdrawal { amount } => {
                let new_available = self.available_funds - amount;
                if new_available < Money::ZERO {
                    // silently fail due to insufficient funds
                    return;
                }
                // note: allows withdrawals from locked/frozen accounts
                if tx_history
                    .record_transaction(tx.id(), amount, tx_history::CompletedTxKind::Withdrawal)
                    .is_err()
                {
                    // silently ignore duplicate transactions
                    return;
                };
                self.available_funds = new_available;
            }
            Dispute => {
                let past_tx = if let Some(past) = tx_history.past_transaction(tx.id()) {
                    past
                } else {
                    // ignore disputes with invalid tx references
                    return;
                };
                use tx_history::CompletedTxKind::*;
                match past_tx.kind {
                    // disputing withdrawals is unsupported.. ignore
                    Withdrawal => return,
                    Deposit => (),
                }
                // this may lead to negative available_funds
                let new_available = self.available_funds - past_tx.amount;
                past_tx.disputed = true;
                self.available_funds = new_available;
                self.held_funds += past_tx.amount;
            }
            Resolve => {
                let past_tx = if let Some(past) = tx_history.past_transaction(tx.id()) {
                    past
                } else {
                    // ignore resolves with invalid tx references
                    return;
                };
                if !past_tx.disputed {
                    // ignore resolves of transactions that aren't disputed
                    return;
                }
                past_tx.disputed = false;
                self.held_funds -= past_tx.amount;
                self.available_funds += past_tx.amount;
            }
            Chargeback => {
                let past_tx = if let Some(past) = tx_history.past_transaction(tx.id()) {
                    past
                } else {
                    // ignore chargebacks with invalid tx references
                    return;
                };
                if !past_tx.disputed {
                    // ignore chargebacks of transactions that aren't disputed
                    return;
                }
                self.held_funds -= past_tx.amount;
                self.locked = true;
                // unwrap won't panic because we already know this entry exists.
                tx_history.erase_transaction(tx.id()).unwrap();
            }
        }
    }
}

impl Serialize for Account {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("Color", 3)?;
        state.serialize_field("client", &self.client)?;
        //state.serialize_field("available",&round_to_four_decimals(self.available_funds))?;
        //state.serialize_field("held",&round_to_four_decimals(self.held_funds))?;
        //state.serialize_field("total",&round_to_four_decimals(self.total()))?;
        state.serialize_field("available", &self.available_funds)?;
        state.serialize_field("held", &self.held_funds)?;
        state.serialize_field("total", &self.total())?;
        state.serialize_field("locked", &self.locked)?;
        state.end()
    }
}
