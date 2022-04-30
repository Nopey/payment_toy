#[cfg(test)]
mod tests;
///! Accounts and operations that can be performed on them
mod transaction;
mod tx_history;

use derive_more::{Add, AddAssign, Display, Sub, SubAssign};
use serde::{ser::SerializeStruct, Deserialize, Serialize};
pub use transaction::{Id as TxId, Transaction};
pub use tx_history::TxHistory;

/// `Client` is an [`Account`]'s unique identifier
pub type Client = u16;

/// `Money` is a numeric quantity with four decimal places.
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

/// `Account` is one's current balance and standing with the bank.
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
    pub fn process_transaction(
        &mut self,
        tx: &Transaction,
        tx_history: &mut TxHistory,
    ) -> Result<(), Error> {
        use transaction::Action::*;
        match tx.action() {
            Deposit { amount } => {
                if self.locked {
                    return Err(Error::AccountLockedFundsFrozen(tx.id()));
                }
                if tx_history
                    .record_transaction(tx.id(), amount, tx_history::CompletedTxKind::Deposit)
                    .is_err()
                {
                    return Err(Error::DuplicateTransaction(tx.id()));
                };
                self.available_funds += amount;
            }
            Withdrawal { amount } => {
                if self.locked {
                    return Err(Error::AccountLockedFundsFrozen(tx.id()));
                }
                let new_available = self.available_funds - amount;
                if new_available < Money::ZERO {
                    return Err(Error::InsufficientFundsForWithdrawal(tx.id()));
                }
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
                if past_tx.disputed {
                    return Err(Error::DuplicateDispute(tx.id()));
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

                // zeroing the deposit's amount prevents repeat chargebacks
                past_tx.amount = Money::ZERO;
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
        state.serialize_field("available", &self.available_funds)?;
        state.serialize_field("held", &self.held_funds)?;
        state.serialize_field("total", &self.total())?;
        state.serialize_field("locked", &self.locked)?;
        state.end()
    }
}

/// An error that occured while processing a transaction
#[derive(Clone, Copy, Debug, PartialEq, Eq)] //, thiserror::Error)]
pub enum Error {
    // #[error("Transaction already exists with id {0}")]
    DuplicateTransaction(TxId),
    // #[error("Transaction {0} attempted to modify funds in locked account")]
    AccountLockedFundsFrozen(TxId),
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
    // #[error("Dispute attempted on transaction {0} that is already in dispute")]
    DuplicateDispute(TxId),
}
