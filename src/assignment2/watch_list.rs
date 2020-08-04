use super::{
    AssignmentError,
    PropagationEnqueuer,
    PropagationResult,
    VariableAssignment,
};
use crate::{
    clause_db::{
        ClauseId,
        PropagationResult as ClausePropagationResult,
    },
    utils::BoundedArray,
    ClauseDb,
    Literal,
    VarAssignment,
    Variable,
};

/// The watchers of a single variable.
///
/// Stores the watchers for the positive and negative polarities of the variable.
#[derive(Debug, Clone, Default)]
pub struct VariableWatchers {
    /// Watchers for the literal with positive polarity.
    pos: Vec<ClauseId>,
    /// Watchers for the literal with negative polarity.
    neg: Vec<ClauseId>,
}

impl VariableWatchers {
    /// Registers the clause identifier for the given literal.
    fn register_for_lit(&mut self, literal: Literal, id: ClauseId) {
        match literal.assignment() {
            VarAssignment::True => self.pos.push(id),
            VarAssignment::False => self.neg.push(id),
        }
    }

    fn literal_watchers_mut(&mut self, literal: Literal) -> &mut Vec<ClauseId> {
        match literal.assignment() {
            VarAssignment::True => &mut self.pos,
            VarAssignment::False => &mut self.neg,
        }
    }

    /// Propagates the literal to the recorded watchers.
    ///
    /// Calls back about the watchers and their propagation results.
    ///
    /// Returns a propagation result that either tells that the propagation
    /// yielded a consistent assignemnt or a conflict.
    fn propagate<F>(
        &mut self,
        literal: Literal,
        clause_db: &mut ClauseDb,
        assignment: &mut VariableAssignment,
        queue: &mut PropagationEnqueuer,
        mut for_watcher: F,
    ) -> PropagationResult
    where
        F: FnMut(ClauseId, ClausePropagationResult),
    {
        println!("VariableWatchers::propagate");
        let mut seen_conflict = false;
        let watchers = self.literal_watchers_mut(literal);
        watchers.retain(|&watcher| {
            if seen_conflict {
                return true
            }
            let result = clause_db
                .resolve_mut(watcher)
                .expect("encountered unexpected invalid clause ID")
                .propagate(literal, &assignment);
            if let ClausePropagationResult::UnitUnderAssignment(unit_literal) = result {
                let enqueue_result = queue.push(unit_literal, assignment);
                if let Err(AssignmentError::Conflict) = enqueue_result {
                    seen_conflict = true;
                }
            }
            let remove_watcher =
                matches!(result, ClausePropagationResult::NewWatchedLiteral(_));
            for_watcher(watcher, result);
            !remove_watcher
        });
        match seen_conflict {
            true => PropagationResult::Conflict,
            false => PropagationResult::Consistent,
        }
    }
}

/// A deferred insertion to the watch list after propagation of a single literal.
#[derive(Debug, Copy, Clone)]
pub struct DeferredWatcherInsert {
    /// The new literal to watch.
    literal: Literal,
    /// The clause that watches the literal.
    watched_by: ClauseId,
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
        self.watchers
            .increase_len_to(total_variables)
            .expect("encountered unexpected invalid size increment");
    }

    /// Registers the clause identifier for the given literal.
    pub fn register_for_lit(&mut self, literal: Literal, clause: ClauseId) {
        self.watchers
            .get_mut(literal.variable())
            .expect("encountered unexpected variable")
            .register_for_lit(literal, clause)
    }

    /// Propagates the literal assignment to the watching clauses.
    pub fn propagate(
        &mut self,
        literal: Literal,
        clause_db: &mut ClauseDb,
        assignment: &mut VariableAssignment,
        mut queue: PropagationEnqueuer<'_>,
    ) -> PropagationResult {
        let Self {
            watchers,
            deferred_inserts,
        } = self;
        println!("WatchList::propagate");
        let result = watchers
            .get_mut(literal.variable())
            .expect("encountered unexpected invalid propagation literal")
            .propagate(
                literal,
                clause_db,
                assignment,
                &mut queue,
                |watcher, result| {
                    if let ClausePropagationResult::NewWatchedLiteral(new_watched) =
                        result
                    {
                        deferred_inserts.push(DeferredWatcherInsert {
                            literal: new_watched,
                            watched_by: watcher,
                        });
                    }
                },
            );
        for deferred in self.deferred_inserts.drain(..) {
            self.watchers
                .get_mut(deferred.literal.variable())
                .expect("encountered unexpected invalid variable")
                .register_for_lit(deferred.literal, deferred.watched_by);
        }
        result
    }
}
