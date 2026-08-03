//! Compensation machinery shared by the mint and the wallet.
//!
//! A saga splits an operation into steps that each commit independently. Because
//! there is no single transaction spanning them, a failed step has to be undone
//! by explicitly reversing the steps that already committed.
//!
//! Steps register a [`CompensatingAction`] as they commit. On failure the
//! registered actions run in LIFO order: a forward path of A, B, C unwinds as C,
//! B, A, because a later step may depend on state an earlier one established.
//!
//! # Context
//!
//! Compensating actions need handles the machinery knows nothing about: the mint
//! reverses proof states through its database and announces the reversal through
//! its pubsub manager, while the wallet works against its local store. Both are
//! supplied through the [`SagaContext`] type parameter, which keeps this module
//! free of any dependency on either side.
//!
//! The type state half of the pattern lives in `cdk::saga`, because the steps it
//! carries are methods on mint and wallet types.

use std::collections::VecDeque;
use std::fmt;

use async_trait::async_trait;

use crate::Error;

/// Handles a [`CompensatingAction`] needs to undo a step.
///
/// Implemented once per side: the mint supplies its database and pubsub manager,
/// the wallet supplies its wallet handle.
pub trait SagaContext: Send + Sync {}

/// A single reversible step, registered once its forward action has committed.
///
/// Implementations must be idempotent. Compensation runs both from the
/// in-process failure path and from crash recovery, so the same action can be
/// applied twice against a database that already reflects part of it.
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait CompensatingAction<C: SagaContext>: Send + Sync {
    /// Undo the step this action was registered for.
    async fn execute(&self, ctx: &C) -> Result<(), Error>;

    /// Name used in compensation logs.
    fn name(&self) -> &'static str;
}

/// Registered compensating actions, most recent first.
pub struct Compensations<C: SagaContext> {
    actions: VecDeque<Box<dyn CompensatingAction<C>>>,
}

impl<C: SagaContext> Default for Compensations<C> {
    fn default() -> Self {
        Self::new()
    }
}

impl<C: SagaContext> Compensations<C> {
    /// Create an empty queue.
    pub fn new() -> Self {
        Self {
            actions: VecDeque::new(),
        }
    }

    /// Register an action to run before every action registered so far.
    pub fn push(&mut self, action: Box<dyn CompensatingAction<C>>) {
        self.actions.push_front(action);
    }

    /// Drop every registered action without running it. Called once the saga has
    /// committed and there is nothing left to undo.
    pub fn clear(&mut self) {
        self.actions.clear();
    }

    /// Whether any action is registered.
    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }

    /// Number of registered actions.
    pub fn len(&self) -> usize {
        self.actions.len()
    }

    /// Run every registered action in LIFO order, draining the queue.
    ///
    /// A failing action is logged and skipped rather than aborting the rest:
    /// leaving the remaining steps un-reversed would strand more state than the
    /// one failure already has.
    pub async fn run(&mut self, ctx: &C) -> Result<(), Error> {
        if self.actions.is_empty() {
            return Ok(());
        }

        tracing::warn!("Running {} compensating actions", self.actions.len());

        while let Some(action) = self.actions.pop_front() {
            tracing::debug!("Running compensation: {}", action.name());
            if let Err(e) = action.execute(ctx).await {
                tracing::error!(
                    "Compensation {} failed: {}. Continuing...",
                    action.name(),
                    e
                );
            }
        }

        Ok(())
    }
}

impl<C: SagaContext> fmt::Debug for Compensations<C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Compensations")
            .field("len", &self.actions.len())
            .finish()
    }
}

/// Test doubles for the saga machinery, shared with `cdk`'s type state tests.
#[cfg(any(test, feature = "test"))]
pub mod test_utils {
    use std::sync::{Arc, Mutex};

    use super::*;

    /// Records the name of every action that ran against it.
    #[derive(Debug)]
    pub struct TestContext {
        /// Names of the actions executed so far, in execution order.
        pub executed: Arc<Mutex<Vec<&'static str>>>,
    }

    impl SagaContext for TestContext {}

    impl TestContext {
        /// Create a context and a handle to the names it will record.
        pub fn new() -> (Self, Arc<Mutex<Vec<&'static str>>>) {
            let executed = Arc::new(Mutex::new(Vec::new()));
            (
                Self {
                    executed: executed.clone(),
                },
                executed,
            )
        }
    }

    /// An action that records its name, and optionally fails.
    #[derive(Debug)]
    pub struct MockAction {
        name: &'static str,
        should_fail: bool,
    }

    impl MockAction {
        /// An action that records its name and succeeds.
        pub fn new(name: &'static str) -> Box<Self> {
            Box::new(Self {
                name,
                should_fail: false,
            })
        }

        /// An action that records its name and then fails.
        pub fn failing(name: &'static str) -> Box<Self> {
            Box::new(Self {
                name,
                should_fail: true,
            })
        }
    }

    #[async_trait]
    impl CompensatingAction<TestContext> for MockAction {
        async fn execute(&self, ctx: &TestContext) -> Result<(), Error> {
            ctx.executed.lock().expect("poisoned").push(self.name);
            if self.should_fail {
                Err(Error::Custom("intentional test failure".to_string()))
            } else {
                Ok(())
            }
        }

        fn name(&self) -> &'static str {
            self.name
        }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::test_utils::{MockAction, TestContext};
    use super::*;

    #[tokio::test]
    async fn compensations_run_in_lifo_order() {
        let (ctx, executed) = TestContext::new();
        let mut compensations = Compensations::new();

        compensations.push(MockAction::new("first"));
        compensations.push(MockAction::new("second"));
        compensations.push(MockAction::new("third"));

        compensations.run(&ctx).await.unwrap();

        assert_eq!(
            executed.lock().unwrap().as_slice(),
            &["third", "second", "first"]
        );
        assert!(compensations.is_empty());
    }

    #[tokio::test]
    async fn compensations_continue_past_a_failure() {
        let (ctx, executed) = TestContext::new();
        let mut compensations = Compensations::new();

        compensations.push(MockAction::new("first"));
        compensations.push(MockAction::failing("second_fails"));
        compensations.push(MockAction::new("third"));

        assert!(compensations.run(&ctx).await.is_ok());

        assert_eq!(
            executed.lock().unwrap().as_slice(),
            &["third", "second_fails", "first"]
        );
    }

    #[tokio::test]
    async fn running_an_empty_queue_succeeds() {
        let (ctx, executed) = TestContext::new();
        let mut compensations = Compensations::<TestContext>::new();

        assert!(compensations.is_empty());
        compensations.run(&ctx).await.unwrap();

        assert!(executed.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn clear_discards_registered_compensations() {
        let (ctx, executed) = TestContext::new();
        let mut compensations = Compensations::new();

        compensations.push(MockAction::new("first"));
        compensations.push(MockAction::new("second"));
        assert_eq!(compensations.len(), 2);

        compensations.clear();
        assert!(compensations.is_empty());

        compensations.run(&ctx).await.unwrap();
        assert!(executed.lock().unwrap().is_empty());
    }
}
