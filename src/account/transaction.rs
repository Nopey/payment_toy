//! `Transaction` represents an operation on an [`Account`](super::Account),
//! that is deserializable from CSV and applied in
//! [`Account::process_transaction`](super::Account::process_transaction)
//!
use super::{Client, Money};
use serde::{de, Deserialize};

pub type Id = u32;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Transaction {
    action: Action,
    client: Client,
    id: Id,
}

impl Transaction {
    #[allow(unused)]
    pub fn new(action: Action, client: Client, id: Id) -> Self {
        Self { action, client, id }
    }
    pub fn action(&self) -> Action {
        self.action
    }
    pub fn client(&self) -> Client {
        self.client
    }
    pub fn id(&self) -> Id {
        self.id
    }
}

// manual impl of Deser for Tx is required because of csv's poor reaction to #[serde(flatten)]
// (csv uses infer_deserialize for the child struct, converting 'amount' to f64)
impl<'de> Deserialize<'de> for Transaction {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct CsvTransaction {
            #[serde(rename = "type")]
            action_type: ActionType,
            amount: Option<Money>,
            client: Client,
            #[serde(rename = "tx")]
            id: Id,
        }
        #[derive(Deserialize)]
        #[serde(rename_all = "lowercase")]
        enum ActionType {
            Deposit,
            Withdrawal,
            Dispute,
            Resolve,
            Chargeback,
        }

        // csv and #[serde(flatten)] don't mix well, so deserialize a flat copy of the struct
        let CsvTransaction {
            action_type,
            mut amount,
            client,
            id,
        } = CsvTransaction::deserialize(deserializer)?;
        // and then un-flatten it
        let mut take_amount = || {
            std::mem::take(&mut amount)
                .ok_or_else(|| de::Error::missing_field("amount"))
                .and_then(|money| {
                    if money.is_negative() {
                        Err(de::Error::invalid_value(
                            serde::de::Unexpected::Float(money.to_f64()),
                            &"a positive amount of moneys",
                        ))
                    } else {
                        Ok(money)
                    }
                })
        };
        let action = match action_type {
            ActionType::Deposit => Action::Deposit {
                amount: take_amount()?,
            },
            ActionType::Withdrawal => Action::Withdrawal {
                amount: take_amount()?,
            },
            ActionType::Dispute => Action::Dispute,
            ActionType::Resolve => Action::Resolve,
            ActionType::Chargeback => Action::Chargeback,
        };
        // whether or not we've called take_amount, amount should now be None.
        if amount.is_some() {
            return Err(de::Error::custom("expected nothing in `amount` field"));
        }

        Ok(Transaction { action, client, id })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Deposit { amount: Money },
    Withdrawal { amount: Money },
    Dispute,
    Resolve,
    Chargeback,
}

#[allow(unused)]
impl Action {
    pub fn new_deposit(amount: Money) -> Self {
        assert!(!amount.is_negative());
        Action::Deposit { amount }
    }
    pub fn new_withdrawal(amount: Money) -> Self {
        assert!(!amount.is_negative());
        Action::Withdrawal { amount }
    }
    pub fn new_dispute() -> Self {
        Action::Dispute
    }
    pub fn new_resolve() -> Self {
        Action::Resolve
    }
    pub fn new_chargeback() -> Self {
        Action::Chargeback
    }
}
