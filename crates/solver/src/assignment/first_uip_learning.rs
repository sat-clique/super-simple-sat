use super::{
    DecisionLevel,
    DecisionLevelsAndReasons,
    Trail,
};
use crate::{
    clause_db::{
        ClauseId,
        ClauseRef,
    },
    ClauseDb,
    Literal,
    Variable,
};
use alloc::vec::Vec;
use bounded::BoundedBitmap;
use core::slice;

/// Types that provide information about the current decision level.
pub trait CurrentDecisionLevel {
    /// Returns the current decision level on the trail.
    fn current_decision_level(&self) -> DecisionLevel;
}

/// Types that provide the assignments of the given decision level.
pub trait LevelAssignments {
    /// Returns all assignments of the current decision level.
    ///
    /// They are in the order in which they have been assigned on the trail.
    fn level_assignments(&self, level: DecisionLevel) -> &[Literal];
}

/// Types that provide resolution of clause IDs into actual clauses.
pub trait ResolveClauseId {
    /// Returns the clause for the given clause ID.
    fn resolve_clause_id(&self, id: ClauseId) -> ClauseRef;
}

pub trait DecisionLevelAndReasonOf {
    fn decision_level_and_reason_of(
        &self,
        variable: Variable,
    ) -> (DecisionLevel, Option<ClauseId>);
}

impl CurrentDecisionLevel for Trail {
    fn current_decision_level(&self) -> DecisionLevel {
        Self::current_decision_level(self)
    }
}

impl LevelAssignments for Trail {
    fn level_assignments(&self, level: DecisionLevel) -> &[Literal] {
        Self::level_assignments(self, level)
    }
}

impl ResolveClauseId for ClauseDb {
    fn resolve_clause_id(&self, id: ClauseId) -> ClauseRef {
        self.resolve(id)
            .expect("encountered unexpected invalid clause ID")
    }
}

impl DecisionLevelAndReasonOf for DecisionLevelsAndReasons {
    fn decision_level_and_reason_of(
        &self,
        variable: Variable,
    ) -> (DecisionLevel, Option<ClauseId>) {
        self.get(variable)
            .expect("encountered missing decision level for variable on the trail")
    }
}

pub struct LearnedClauseLiterals<'a> {
    literals: slice::Iter<'a, Literal>,
}

impl<'a> LearnedClauseLiterals<'a> {
    /// Creates a new learned clause literals iterator from the given literals buffer.
    fn new(literals: &'a [Literal]) -> Self {
        Self {
            literals: literals.iter(),
        }
    }
}

impl<'a> Iterator for LearnedClauseLiterals<'a> {
    type Item = Literal;

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.literals.size_hint()
    }

    fn next(&mut self) -> Option<Self::Item> {
        self.literals.next().copied()
    }
}

impl<'a> ExactSizeIterator for LearnedClauseLiterals<'a> {}

#[derive(Debug, Default, Clone)]
struct StampMap {
    stamps: BoundedBitmap<Variable, bool>,
}

impl StampMap {
    /// Returns the current number of registered variables.
    fn len_variables(&self) -> usize {
        self.stamps.len()
    }

    /// Registers the given number of additional variables.
    ///
    /// # Panics
    ///
    /// If the number of total variables is out of supported bounds.
    pub fn register_new_variables(&mut self, new_variables: usize) {
        let total_variables = self.len_variables() + new_variables;
        self.stamps.resize_to_len(total_variables);
    }

    /// Stamps the given variable.
    ///
    /// # Panics
    ///
    /// If the variable is invalid.
    pub fn stamp(&mut self, variable: Variable) {
        self.stamps
            .set(variable, true)
            .expect("encountered unexpected invalid variable upon stamping");
    }

    /// Unstamps the given variable.
    ///
    /// # Panics
    ///
    /// If the variable is invalid.
    pub fn unstamp(&mut self, variable: Variable) {
        self.stamps
            .set(variable, false)
            .expect("encountered unexpected invalid variable upon unstamping");
    }

    /// Returns `true` if the variable has been stamped.
    pub fn is_stamped(&self, variable: Variable) -> bool {
        self.stamps
            .get(variable)
            .expect("encountered unexpected invalid variable upon querying stamp state")
    }
}

#[derive(Debug, Default, Clone)]
pub struct FirstUipLearning {
    /// Temporary storage for stamps since we cannot afford to allocate (and initialize) a
    /// new vector with the length of the number of variables each time a conflict clause
    /// is computed.
    /// This member variable is governed by class invariant `A`.
    ///
    /// A variable `v` can be "stamped" for two reasons:
    ///
    /// - if `v` has been assigned on the current decision level: when traversing down the trail,
    ///   resolution with `v`'s reason clause needs to be performed.
    /// - if `v` has not been assigned on the current decision level: `v` occurs in the result.
    ///
    /// Note that only one variable assigned on the current decision level can actually occur
    /// in the result: the variable of the asserting literal (UIP).
    ///
    /// Thus, `stamps` is used both for keeping track of remaining resolution work, and
    /// for quickly deciding whether a variable already occurs in the result. These two concerns
    /// are handled by the same data structure for memory efficiency.
    stamps: StampMap,
    /// Temporary buffer to store literals of the learned clauses.
    result: Vec<Literal>,
}

impl FirstUipLearning {
    /// Registers the given number of additional variables.
    ///
    /// # Panics
    ///
    /// If the number of total variables is out of supported bounds.
    pub fn register_new_variables(&mut self, new_variables: usize) {
        self.stamps.register_new_variables(new_variables);
    }

    /// Given a conflicting clause computes the conflict clause.
    ///
    /// Returns an iterator over the literals of the learned clause.
    /// The asserting literal is yielded first.
    pub fn compute_conflict_clause<T, R, C>(
        &mut self,
        conflicting_clause: ClauseRef,
        trail: &T,
        levels_and_reasons: &R,
        clause_db: &C,
    ) -> LearnedClauseLiterals
    where
        T: CurrentDecisionLevel + LevelAssignments,
        R: DecisionLevelAndReasonOf,
        C: ResolveClauseId,
    {
        let count_unresolved =
            self.initialze_result(conflicting_clause, trail, levels_and_reasons);
        self.resolve_until_uip(count_unresolved, trail, levels_and_reasons, clause_db);
        self.clear_stamps();
        LearnedClauseLiterals::new(self.result.as_slice())
    }

    /// Resets the stamps for the variables of the given literals.
    fn clear_stamps(&mut self) {
        for literal in &self.result {
            self.stamps.unstamp(literal.variable());
        }
    }

    /// Initializes the result buffer.
    ///
    /// The result buffer is initialized by inserting all literals of the
    /// given conflicting clause that occur on another decision level than the
    /// current one.
    /// Furthermore, if the asserting literal is found during this call, it is
    /// prepended to the result buffer. Note that this can happen if the
    /// current decision level's decision literal occurs in the conflicting clause.
    /// Otherwise, the undefined literal is prepended to the result buffer.
    ///
    /// All literals contained in the conflicting clause are stamped.
    ///
    /// When this methid is invoked, class invariant A needs to hold. When this
    /// method returns, `stamps[v] == 1` is true if and only if a literal occurs
    /// in the result buffer with variable `v or v` is the variable of a literal
    /// at which resolution needs to be performed. (??)
    ///
    /// Returns the amount of literals on the current decision level found in the
    /// conflicting clause.
    fn initialze_result<T, R>(
        &mut self,
        conflicting_clause: ClauseRef,
        trail: &T,
        levels_and_reasons: &R,
    ) -> usize
    where
        T: CurrentDecisionLevel,
        R: DecisionLevelAndReasonOf,
    {
        self.result.clear();
        // Mark the literals on the current decision levels as work, put
        // the rest into the result, stamp them all - this can be done
        // by resolving the conflicting clause with an empty clause and
        // adding an imaginary literal L rsp. ~L to the two clauses. The
        // imaginary literal is `None`, in this case.
        let count_unresolved =
            self.add_resolvent(conflicting_clause, None, trail, levels_and_reasons);
        // If count_unresolved == 1, the single literal on the current decision level
        // would have gotten a forced assignment on a lower decision level, which
        // is impossible. If count_unresolved == 0, the clause has no literals
        // on the current decision level and could not have been part of the
        // conflict in the first place, either.
        assert!(
            count_unresolved >= 2,
            "encountered fewer than 2 literals on the current decision level during first-UIP initialization"
        );
        count_unresolved
    }

    /// Completes the result buffer to be resolved with `reason` at `resolve_at_lit`
    /// omitting the literals of the current decision level in the result buffer.
    ///
    /// # Developer Note
    ///
    /// In the following `work` is the set of literals whose variable has been assigned
    /// on the current deciision level and which have been encountered so far during
    /// the resolution process.
    ///
    /// When this method is invoked, it must hold that `stamps[v] == 1` if and only if
    /// `v` is the variable of a literal occuring in the result buffer or `v` is the
    /// variable of a literal at which resolution still remains to be performed.
    /// This also holds when this method returns.
    ///
    /// Returns the amount of literals added to `work`.
    fn add_resolvent<T, R>(
        &mut self,
        reason: ClauseRef,
        resolve_at_lit: Option<Literal>,
        trail: &T,
        levels_and_reasons: &R,
    ) -> usize
    where
        T: CurrentDecisionLevel,
        R: DecisionLevelAndReasonOf,
    {
        // Stamp literals on the current decision level and mark them as resolution
        // "work". All others already belong to the result: resolution is not
        // performed at these literals, since none of their inverses can appear in
        // reason clauses for variables on the current decision level. They may
        // appear in those reason clauses with the same sign, though, which is why
        // we need to keep track of the literals already included in the result.
        if let Some(resolve_at_lit) = resolve_at_lit {
            debug_assert!(self.stamps.is_stamped(resolve_at_lit.variable()));
            self.stamps.unstamp(resolve_at_lit.variable());
        }
        // Reserve upfront in the result buffer for the reason clause literals.
        self.result.reserve(reason.len());
        let current_level = trail.current_decision_level();
        let mut count_unresolved = 0;
        for reason_literal in reason {
            let reason_variable = reason_literal.variable();
            if Some(reason_literal) != resolve_at_lit
                && !self.stamps.is_stamped(reason_variable)
            {
                self.stamps.stamp(reason_variable);
                let (reason_level, _) =
                    levels_and_reasons.decision_level_and_reason_of(reason_variable);
                if reason_level == current_level {
                    count_unresolved += 1;
                } else {
                    self.result.push(reason_literal);
                }
            }
        }
        count_unresolved
    }

    /// Finds the 1-UIP for the given level assignments, trail and reasons.
    ///
    /// Should be followed with a call to [`Self::find_asserting_literal`].
    ///
    /// # Panics
    ///
    /// - If the 1-UIP has been found too early.
    /// - If the 1-UIP has not been found at all.
    fn find_first_uip<T, R, C, L>(
        &mut self,
        count_unresolved: usize,
        level_assignments: &mut L,
        trail: &T,
        levels_and_reasons: &R,
        clause_db: &C,
    ) where
        T: CurrentDecisionLevel,
        R: DecisionLevelAndReasonOf,
        C: ResolveClauseId,
        L: Iterator<Item = Literal>,
    {
        let mut count_unresolved = count_unresolved;
        let current_level = trail.current_decision_level();
        while count_unresolved != 1 {
            let resolve_at_lit = level_assignments
                .next()
                .expect("encountered unexpected missing level assignment");
            let resolve_at_var = resolve_at_lit.variable();
            let is_stamped = self.stamps.is_stamped(resolve_at_var);
            if is_stamped {
                let (level, reason) =
                    levels_and_reasons.decision_level_and_reason_of(resolve_at_var);
                debug_assert_eq!(level, current_level);
                match reason {
                    None => panic!("encountered the 1-UIP too early"),
                    Some(reason) => {
                        let reason = clause_db.resolve_clause_id(reason);
                        count_unresolved += self.add_resolvent(
                            reason,
                            Some(resolve_at_lit),
                            trail,
                            levels_and_reasons,
                        );
                        count_unresolved -= 1;
                    }
                }
            }
        }
        assert_eq!(
            count_unresolved, 1,
            "reached the end of the decision level assignments without finding the 1-UIP"
        );
    }

    /// Finds the asserting literal.
    ///
    /// Needs to be called after [`Self::find_first_uip`].
    /// Places the asserting literal into the first position of the result buffer.
    fn find_asserting_literal<L>(&mut self, level_assignments: &mut L)
    where
        L: Iterator<Item = Literal>,
    {
        let asserting_literal = level_assignments
            .find(|literal| {
                let var = literal.variable();
                self.stamps.is_stamped(var)
            })
            .expect("encountered missing asserting literal");
        self.result.push(asserting_literal);
        // Swap first and last to put the asserting literal into the first position.
        let last = self.result.len() - 1;
        self.result.swap(0, last);
        self.stamps.unstamp(asserting_literal.variable());
    }

    /// Iteratively resolves the result buffer with reason clause of literals
    /// occurring on the current decision level, aborting when having reached the
    /// first unique implication point.
    fn resolve_until_uip<T, R, C>(
        &mut self,
        count_unresolved: usize,
        trail: &T,
        levels_and_reasons: &R,
        clause_db: &C,
    ) where
        T: CurrentDecisionLevel + LevelAssignments,
        R: DecisionLevelAndReasonOf,
        C: ResolveClauseId,
    {
        let current_level = trail.current_decision_level();
        let mut level_assignments = trail
            .level_assignments(current_level)
            .into_iter()
            .copied()
            .rev();
        self.find_first_uip(
            count_unresolved,
            &mut level_assignments,
            trail,
            levels_and_reasons,
            clause_db,
        );
        self.find_asserting_literal(&mut level_assignments);
    }
}
