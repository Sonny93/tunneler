//! Holds the live daemon connection, guarded against reconnect races.

use std::sync::{Arc, Mutex, PoisonError};

/// Identifies one installed connection; a later install gets a greater one.
pub type Generation = u64;

struct Installed<T> {
    generation: Generation,
    value: Arc<T>,
}

struct State<T> {
    last_generation: Generation,
    installed: Option<Installed<T>>,
}

/// The current connection, if any.
///
/// Every install gets a new generation, so the relay of a connection that
/// ended can clear the slot only while its own connection is still the one in
/// it.
pub struct ConnectionSlot<T> {
    state: Mutex<State<T>>,
}

impl<T> Default for ConnectionSlot<T> {
    fn default() -> Self {
        Self {
            state: Mutex::new(State {
                last_generation: 0,
                installed: None,
            }),
        }
    }
}

impl<T> ConnectionSlot<T> {
    /// The installed connection, if any.
    pub fn current(&self) -> Option<Arc<T>> {
        self.lock()
            .installed
            .as_ref()
            .map(|installed| Arc::clone(&installed.value))
    }

    /// Replaces the installed connection and returns its generation.
    pub fn install(&self, value: Arc<T>) -> Generation {
        let mut state = self.lock();
        state.last_generation += 1;
        let generation = state.last_generation;
        state.installed = Some(Installed { generation, value });
        generation
    }

    /// Empties the slot when it still holds `generation`; tells whether it did.
    pub fn clear_if_current(&self, generation: Generation) -> bool {
        let mut state = self.lock();
        let is_current = state
            .installed
            .as_ref()
            .is_some_and(|installed| installed.generation == generation);
        if is_current {
            state.installed = None;
        }
        is_current
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, State<T>> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_hold_nothing_when_nothing_was_installed() {
        let slot = ConnectionSlot::<u8>::default();

        assert!(slot.current().is_none());
    }

    #[test]
    fn should_return_the_installed_value_when_installed() {
        let slot = ConnectionSlot::default();
        slot.install(Arc::new(7_u8));

        assert_eq!(slot.current().as_deref(), Some(&7));
    }

    #[test]
    fn should_empty_the_slot_when_clearing_the_current_generation() {
        let slot = ConnectionSlot::default();
        let generation = slot.install(Arc::new(7_u8));

        assert!(slot.clear_if_current(generation));
        assert!(slot.current().is_none());
    }

    #[test]
    fn should_keep_the_new_connection_when_clearing_a_superseded_generation() {
        let slot = ConnectionSlot::default();
        let old_generation = slot.install(Arc::new(1_u8));
        slot.install(Arc::new(2_u8));

        assert!(!slot.clear_if_current(old_generation));
        assert_eq!(slot.current().as_deref(), Some(&2));
    }

    #[test]
    fn should_not_clear_twice_when_the_generation_was_already_cleared() {
        let slot = ConnectionSlot::default();
        let generation = slot.install(Arc::new(1_u8));
        slot.clear_if_current(generation);

        assert!(!slot.clear_if_current(generation));
    }
}
