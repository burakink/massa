// Copyright (c) 2022 MASSA LABS <info@massa.net>

use std::sync::Arc;

use massa_models::{prehash::Map, Address, Amount, Slot};
use massa_pos_exports::{PoSChanges, SelectorController};
use parking_lot::RwLock;

use crate::active_history::ActiveHistory;

/// Speculative state of the rolls
#[allow(dead_code)]
pub(crate) struct SpeculativeRollState {
    /// Selector used to feed_cycle and get_selection
    selector: Box<dyn SelectorController>,
    /// History of the outputs of recently executed slots.
    /// Slots should be consecutive, newest at the back.
    active_history: Arc<RwLock<ActiveHistory>>,
    /// List of changes to the state after settling roll sell/buy
    added_changes: PoSChanges,
}

impl SpeculativeRollState {
    /// Creates a new `SpeculativeRollState`
    ///
    /// # Arguments
    /// * `selector`: PoS draws selector controller
    /// * `active_history`: thread-safe shared access the speculative execution history
    pub fn new(
        selector: Box<dyn SelectorController>,
        active_history: Arc<RwLock<ActiveHistory>>,
    ) -> Self {
        SpeculativeRollState {
            selector,
            active_history,
            added_changes: Default::default(),
        }
    }

    /// Returns the changes caused to the `SpeculativeRollState` since its creation,
    /// and resets their local value to nothing.
    pub fn take(&mut self) -> PoSChanges {
        std::mem::take(&mut self.added_changes)
    }

    /// Takes a snapshot (clone) of the changes caused to the `SpeculativeRollState` since its creation
    pub fn get_snapshot(&self) -> PoSChanges {
        self.added_changes.clone()
    }

    /// Resets the `SpeculativeRollState` to a snapshot (see `get_snapshot` method)
    pub fn reset_to_snapshot(&mut self, snapshot: PoSChanges) {
        self.added_changes = snapshot;
    }

    /// Add `roll_count` rolls to the given address
    pub fn add_rolls(&mut self, buyer_addr: &Address, roll_count: u64) {
        // NOTE: probably want to move this to context
        let count = self
            .added_changes
            .roll_changes
            .entry(*buyer_addr)
            .or_insert(
                self.active_history
                    .read()
                    .fetch_roll_count(buyer_addr)
                    .unwrap_or_default(),
            );
        *count = count.saturating_add(roll_count);
    }

    /// Remove `roll_count` rolls from the given address and add deferred reimbursement
    pub fn remove_rolls(
        &mut self,
        seller_addr: &Address,
        slot: Slot,
        roll_price: Amount,
        roll_count: u64,
    ) {
        // NOTE: probably want to move this to context
        let count = self
            .added_changes
            .roll_changes
            .entry(*seller_addr)
            .or_insert(
                self.active_history
                    .read()
                    .fetch_roll_count(seller_addr)
                    .unwrap_or_default(),
            );
        *count = count.saturating_sub(roll_count);
        let mut a: Map<Address, Amount> = Map::default();
        a.insert(*seller_addr, roll_price.saturating_mul_u64(roll_count));
        let _b = self.added_changes.deferred_credits.entry(slot).or_insert(a);
    }

    /// Update the production stats
    #[allow(dead_code)]
    pub fn update_production_stats(
        &mut self,
        creator: &Address,
        slot: &Slot,
        contains_block: bool,
    ) {
        // note: will be used only on real execution
        if let Some(production_stats) = self.added_changes.production_stats.get_mut(creator) {
            if contains_block {
                production_stats.block_success_count =
                    production_stats.block_success_count.saturating_add(1);
                self.added_changes.seed_bits.push(slot.get_first_bit());
            } else {
                production_stats.block_failure_count =
                    production_stats.block_failure_count.saturating_add(1);
            }
        }
    }

    /// Settle a slot.
    ///
    /// Compute the changes to be made on the roll state at the end of a given slot.
    pub fn settle_slot(&mut self, _slot: Slot) {
        // note: will be used on every kind of execution
    }
}