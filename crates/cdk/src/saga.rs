//! Type state for sagas, shared by the mint and the wallet.
//!
//! [`Saga`] carries its progress in a type parameter, so a step that is not
//! legal yet does not exist as a method. Each step consumes the saga and returns
//! it in the next state via [`Saga::advance`], which carries the context and any
//! compensations registered so far into that next state.
//!
//! The steps themselves are inherent methods on `Saga<C, S>` for the concrete
//! context and state of each operation, defined next to the operation they
//! belong to. That is why this type lives here rather than in `cdk-common`
//! alongside [`CompensatingAction`]: inherent impls have to sit in the crate
//! that defines the type, and both the mint and the wallet live in this one.

use std::fmt;

use cdk_common::saga::{CompensatingAction, Compensations, SagaContext};
use uuid::Uuid;

use crate::Error;

/// An operation in progress, in state `S`, against context `C`.
pub struct Saga<C: SagaContext, S> {
    /// Handles the saga and its compensating actions operate against.
    pub ctx: C,
    /// Identifies this saga in the database and on the proofs it reserved.
    pub operation_id: Uuid,
    /// Data belonging to the current step.
    pub state: S,
    compensations: Compensations<C>,
}

impl<C: SagaContext, S> Saga<C, S> {
    /// Start a saga in its initial state.
    ///
    /// Named `start` rather than `new` so each operation can keep its own
    /// `new()` constructor on the same type.
    pub fn start(ctx: C, operation_id: Uuid, state: S) -> Self {
        Self {
            ctx,
            operation_id,
            state,
            compensations: Compensations::new(),
        }
    }

    /// Move to the next state, carrying the context and registered compensations.
    pub fn advance<T>(self, state: T) -> Saga<C, T> {
        Saga {
            ctx: self.ctx,
            operation_id: self.operation_id,
            state,
            compensations: self.compensations,
        }
    }

    /// Move to the next state, building it from the current one.
    ///
    /// [`Self::advance`] cannot be used when the next state takes ownership of
    /// fields from the current one, because moving out of `self.state` leaves
    /// `self` unusable as the receiver.
    pub fn map_state<T>(self, next: impl FnOnce(S) -> T) -> Saga<C, T> {
        Saga {
            ctx: self.ctx,
            operation_id: self.operation_id,
            state: next(self.state),
            compensations: self.compensations,
        }
    }

    /// Take the current state by value, keeping the saga usable.
    ///
    /// For a step that consumes its state but still has to clear or run
    /// compensations depending on how it turns out.
    pub fn take_state(self) -> (Saga<C, ()>, S) {
        let Saga {
            ctx,
            operation_id,
            state,
            compensations,
        } = self;
        (
            Saga {
                ctx,
                operation_id,
                state: (),
                compensations,
            },
            state,
        )
    }

    /// Register an action to undo the step that just committed.
    pub fn push_compensation(&mut self, action: Box<dyn CompensatingAction<C>>) {
        self.compensations.push(action);
    }

    /// Discard the registered compensations because the saga completed and there
    /// is nothing left to undo.
    pub fn clear_compensations(&mut self) {
        self.compensations.clear();
    }

    /// The compensations registered so far.
    pub fn compensations(&self) -> &Compensations<C> {
        &self.compensations
    }

    /// Undo every committed step, consuming the saga so it cannot be used after
    /// being rolled back.
    pub async fn compensate(self) -> Result<(), Error> {
        let Saga {
            ctx,
            mut compensations,
            ..
        } = self;
        compensations.run(&ctx).await
    }
}

/// Contexts hold database and pubsub handles that are not `Debug`, so only the
/// saga's own progress is printed.
impl<C: SagaContext, S: fmt::Debug> fmt::Debug for Saga<C, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Saga")
            .field("operation_id", &self.operation_id)
            .field("state", &self.state)
            .field("compensations", &self.compensations)
            .finish()
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use cdk_common::saga::test_utils::{MockAction, TestContext};

    use super::*;

    #[tokio::test]
    async fn advance_carries_operation_id_and_compensations() {
        let (ctx, executed) = TestContext::new();
        let operation_id = Uuid::new_v4();
        let mut saga = Saga::start(ctx, operation_id, "first state");

        saga.push_compensation(MockAction::new("from_first_state"));

        let saga = saga.advance("second state");

        assert_eq!(saga.operation_id, operation_id);
        assert_eq!(saga.state, "second state");
        assert!(!saga.compensations().is_empty());

        saga.compensate().await.unwrap();
        assert_eq!(executed.lock().unwrap().as_slice(), &["from_first_state"]);
    }

    #[tokio::test]
    async fn clearing_compensations_leaves_nothing_to_undo() {
        let (ctx, executed) = TestContext::new();
        let mut saga = Saga::start(ctx, Uuid::new_v4(), ());

        saga.push_compensation(MockAction::new("registered"));
        assert!(!saga.compensations().is_empty());

        saga.clear_compensations();
        assert!(saga.compensations().is_empty());

        saga.compensate().await.unwrap();
        assert!(executed.lock().unwrap().is_empty());
    }
}
