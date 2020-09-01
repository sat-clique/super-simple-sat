use super::{
    AssignmentError,
    DecisionLevelsAndReasons,
    PropagationEnqueuer,
    PropagationResult,
    VariableAssignment,
};
use crate::{
    clause_db::{
        ClauseId,
        PropagationResult as ClausePropagationResult,
    },
    ClauseDb,
    Literal,
    Sign,
    Variable,
};
use alloc::vec::Vec;
use bounded::BoundedArray;

/// Registered watcher for a single literal with a blocker literal.
///
/// # Note
///
/// When the blocker literal is `true` under the current assignment the watcher
/// does not need to be looked-up which is a relatively costly operation.
#[derive(Debug, Copy, Clone)]
struct Watcher {
    blocker: Literal,
    watcher: ClauseId,
}

impl Watcher {
    /// Creates a new watcher from the given blocker literal and watcher.
    pub fn new(blocker: Literal, watcher: ClauseId) -> Self {
        Self { blocker, watcher }
    }
}

/// The watchers of a single variable.
///
/// Stores the watchers for the positive and negative polarities of the variable.
#[derive(Debug, Clone, Default)]
pub struct VariableWatchers {
    /// Watchers for the literal with positive polarity.
    pos: Vec<Watcher>,
    /// Watchers for the literal with negative polarity.
    neg: Vec<Watcher>,
}

impl VariableWatchers {
    /// Registers the clause identifier for the given literal.
    fn register_for_lit(
        &mut self,
        watched: Literal,
        blocker: Literal,
        watcher: ClauseId,
    ) {
        let watcher = Watcher::new(blocker, watcher);
        match watched.assignment() {
            Sign::True => self.pos.push(watcher),
            Sign::False => self.neg.push(watcher),
        }
    }

    fn literal_watchers_mut(&mut self, literal: Literal) -> &mut Vec<Watcher> {
        match literal.assignment() {
            Sign::True => &mut self.pos,
            Sign::False => &mut self.neg,
        }
    }

    /// Propagates the literal to the recorded watchers.
    ///
    /// Calls back about the watchers and their propagation results.
    ///
    /// Returns a propagation result that either tells that the propagation
    /// yielded a consistent assignemnt or a conflict.
    fn propagate(
        &mut self,
        literal: Literal,
        clause_db: &mut ClauseDb,
        assignment: &mut VariableAssignment,
        levels_and_decisions: &mut DecisionLevelsAndReasons,
        queue: &mut PropagationEnqueuer,
        mut watcher_enqueue: EnqueueWatcher,
    ) -> PropagationResult {
        let mut seen_conflict = false;
        let watchers = self.literal_watchers_mut(literal);
        watchers.retain(|&watcher| {
            // Closure returns `false` if the watcher needs to be removed.
            if seen_conflict {
                return true
            }
            if let Some(true) = assignment.is_satisfied(watcher.blocker) {
                // Skip clause look-up if the blocker is already satisfied.
                return true
            }
            let watcher = watcher.watcher;
            let result = clause_db
                .resolve_mut(watcher)
                .expect("encountered unexpected invalid clause ID")
                .propagate(literal, &assignment);
            match result {
                ClausePropagationResult::UnitUnderAssignment(unit_literal) => {
                    let enqueue_result = queue.push(unit_literal, Some(watcher), assignment, levels_and_decisions);
                    if let Err(AssignmentError::Conflict) = enqueue_result {
                        seen_conflict = true;
                    }
                    true
                }
                ClausePropagationResult::NewWatchedLiteral {
                    new_watched,
                    new_blocker,
                } => {
                    watcher_enqueue.enqueue(new_watched, new_blocker, watcher);
                    false
                }
            }
        });
        match seen_conflict {
            true => PropagationResult::Conflict,
            false => PropagationResult::Consistent,
        }
    }
}

/// A deferred insertion to the watch list after propagation of a single literal.
#[derive(Debug, Copy, Clone)]
struct DeferredWatcherInsert {
    /// The new literal to watch.
    watched: Literal,
    /// The blocking literal.
    blocker: Literal,
    /// The clause that watches the literal.
    watched_by: ClauseId,
}

/// Wrapper around the deferred watcher insertions.
struct EnqueueWatcher<'a> {
    queue: &'a mut Vec<DeferredWatcherInsert>,
}

impl<'a> EnqueueWatcher<'a> {
    /// Creates a new enqueue watcher for the given queue.
    fn new(queue: &'a mut Vec<DeferredWatcherInsert>) -> Self {
        Self { queue }
    }

    /// Enqueues another new watched literal insertion into the queue.
    pub fn enqueue(&mut self, watched: Literal, blocker: Literal, watcher: ClauseId) {
        self.queue.push(DeferredWatcherInsert {
            watched,
            blocker,
            watched_by: watcher,
        });
    }
}

/// The watch list monitoring which clauses are watching which literals.
#[derive(Debug, Default, Clone)]
pub struct WatchList {
    deferred_inserts: Vec<DeferredWatcherInsert>,
    watchers: BoundedArray<Variable, VariableWatchers>,
}

impl WatchList {
    /// Returns the current number of registered variables.
    fn len_variables(&self) -> usize {
        self.watchers.len()
    }

    /// Registers the given number of additional variables.
    ///
    /// # Errors
    ///
    /// If the number of total variables is out of supported bounds.
    pub fn register_new_variables(&mut self, new_variables: usize) {
        let total_variables = self.len_variables() + new_variables;
        self.watchers.resize_with(total_variables, Default::default);
    }

    /// Registers the clause identifier for the given literal.
    pub fn register_for_lit(
        &mut self,
        watched: Literal,
        blocker: Literal,
        watcher: ClauseId,
    ) {
        self.watchers
            .get_mut(watched.variable())
            .expect("encountered unexpected variable")
            .register_for_lit(watched, blocker, watcher)
    }

    /// Propagates the literal assignment to the watching clauses.
    pub fn propagate(
        &mut self,
        literal: Literal,
        clause_db: &mut ClauseDb,
        assignment: &mut VariableAssignment,
        levels_and_reasons: &mut DecisionLevelsAndReasons,
        mut queue: PropagationEnqueuer<'_>,
    ) -> PropagationResult {
        let Self {
            watchers,
            deferred_inserts,
        } = self;
        let result = watchers
            .get_mut(literal.variable())
            .expect("encountered unexpected invalid propagation literal")
            .propagate(
                literal,
                clause_db,
                assignment,
                levels_and_reasons,
                &mut queue,
                EnqueueWatcher::new(deferred_inserts),
            );
        for deferred in deferred_inserts.drain(..) {
            watchers
                .get_mut(deferred.watched.variable())
                .expect("encountered unexpected invalid variable")
                .register_for_lit(
                    deferred.watched,
                    deferred.blocker,
                    deferred.watched_by,
                );
        }
        result
    }
}
