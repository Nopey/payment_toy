mod transaction;
mod tx_history;
#[cfg(test)]
mod tests;

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
    Eq,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
)]
#[serde(transparent)]
pub struct Money(#[serde(with = "rust_decimal::serde::str")] rust_decimal::Decimal);

impl Money {
    pub const ZERO: Money = Money(rust_decimal::Decimal::ZERO);

    #[cfg(test)]
    fn from_i128(num: i128) -> Self {
        Money(rust_decimal::Decimal::from_i128_with_scale(num, 4))
    }
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
    pub fn process_transaction(&mut self, tx: &Transaction, tx_history: &mut TxHistory) -> Result<(), Error>{
        use transaction::Action::*;
        match tx.action() {
            Deposit { amount } => {
                if tx_history
                    .record_transaction(tx.id(), amount, tx_history::CompletedTxKind::Deposit)
                    .is_err()
                {
                    return Err(Error::DuplicateTransaction(tx.id()));
                };
                self.available_funds += amount;
            }
            Withdrawal { amount } => {
                let new_available = self.available_funds - amount;
                if new_available < Money::ZERO {
                    return Err(Error::InsufficientFundsForWithdrawal(tx.id()));
                }
                // note: allows withdrawals from locked/frozen accounts
                if tx_history
                    .record_transaction(tx.id(), amount, tx_history::CompletedTxKind::Withdrawal)
                    .is_err()
                {
                    return Err(Error::DuplicateTransaction(tx.id()));
                };
                self.available_funds = new_available;
            }
            Dispute => {
                let past_tx = if let Some(past) = tx_history.past_transaction(tx.id()) {
                    past
                } else {
                    return Err(Error::UnknownTxReference(tx.id()));
                };
                use tx_history::CompletedTxKind::*;
                match past_tx.kind {
                    // disputing withdrawals is unsupported.. ignore
                    Withdrawal => return Err(Error::WithdrawalsAreIndisputable(tx.id())),
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
                    return Err(Error::UnknownTxReference(tx.id()));
                };
                if !past_tx.disputed {
                    return Err(Error::CantResolveIndisputedTx(tx.id()));
                }
                past_tx.disputed = false;
                self.held_funds -= past_tx.amount;
                self.available_funds += past_tx.amount;
            }
            Chargeback => {
                let past_tx = if let Some(past) = tx_history.past_transaction(tx.id()) {
                    past
                } else {
                    return Err(Error::UnknownTxReference(tx.id()));
                };
                if !past_tx.disputed {
                    return Err(Error::CantChargebackIndisputedTx(tx.id()));
                }
                self.held_funds -= past_tx.amount;
                self.locked = true;
                // unwrap won't panic because we already know this entry exists.
                tx_history.erase_transaction(tx.id()).unwrap();
            }
        }
        Ok(())
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]//, thiserror::Error)]
pub enum Error {
    // #[error("Transaction already exists with id {0}")]
    DuplicateTransaction(TxId),
    // #[error("Insufficient funds for withdrawal in tx {0}")]
    InsufficientFundsForWithdrawal(TxId),
    // #[error("Unknown transaction {0} referenced")]
    UnknownTxReference(TxId),
    // #[error("Disputing withdrawals is unsupported. tx: {0}")]
    WithdrawalsAreIndisputable(TxId),
    // #[error("Resolve attempted on indisupted transaction {0}")]
    CantResolveIndisputedTx(TxId),
    // #[error("Chargeback attempted on indisupted transaction {0}")]
    CantChargebackIndisputedTx(TxId),
}
