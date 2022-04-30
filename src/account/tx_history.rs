//! A history of account deposits and withdrawals to facilitate disputes and chargebacks.
//!
use super::{Money, TxId};
use std::collections::HashMap;

#[derive(Default)]
pub struct TxHistory(HashMap<TxId, CompletedTx>);

impl TxHistory {
    pub(super) fn record_transaction(
        &mut self,
        id: TxId,
        amount: Money,
        kind: CompletedTxKind,
    ) -> Result<(), ()> {
        let entry = self.0.entry(id);
        use std::collections::hash_map::Entry::*;
        match entry {
            Occupied(_) => Err(()),
            Vacant(v) => {
                v.insert(CompletedTx {
                    kind,
                    amount,
                    disputed: false,
                });
                Ok(())
            }
        }
    }

    pub(super) fn past_transaction(&mut self, id: TxId) -> Option<&mut CompletedTx> {
        self.0.get_mut(&id)
    }
}

pub(super) struct CompletedTx {
    pub kind: CompletedTxKind,
    pub amount: Money,
    pub disputed: bool,
}

pub(super) enum CompletedTxKind {
    Withdrawal,
    Deposit,
}
