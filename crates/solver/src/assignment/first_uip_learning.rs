use super::{
    DecisionLevelsAndReasons,
    Trail,
};
use crate::{
    clause_db::ClauseRef,
    ClauseDb,
    Literal,
    Variable,
};
use alloc::vec::Vec;
use bounded::BoundedBitmap;
use core::slice;

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
    result: Vec<Option<Literal>>,
}

pub struct LearnedClauseLiterals<'a> {
    literals: slice::Iter<'a, Option<Literal>>,
}

impl<'a> LearnedClauseLiterals<'a> {
    /// Creates a new learned clause literals iterator from the given literals buffer.
    ///
    /// # Panics
    ///
    /// If the literals buffer contains undefined literals (`None`).
    fn new(literals: &'a [Option<Literal>]) -> Self {
        debug_assert!(
            literals.iter().all(|literal| literal.is_some()),
            "the conflict clause still contains some undefined literals after the 1-UIP resolution"
        );
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
        match self.literals.next() {
            Some(Some(literal)) => Some(*literal),
            None => None,
            Some(None) => {
                unreachable!("encountered unexpected undefined literal in learned clause")
            }
        }
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
    pub fn compute_conflict_clause(
        &mut self,
        conflicting_clause: ClauseRef,
        trail: &Trail,
        levels_and_reasons: &DecisionLevelsAndReasons,
        clause_db: &ClauseDb,
    ) -> LearnedClauseLiterals {
        let count_unresolved =
            self.initialze_result(conflicting_clause, trail, levels_and_reasons);
        self.resolve_until_uip(count_unresolved, trail, levels_and_reasons, clause_db);
        self.clear_stamps();
        LearnedClauseLiterals::new(self.result.as_slice())
    }

    /// Initializes the result buffer.
    ///
    /// The result buffer is initialized by inserting all literals of the
    /// given conflicting clause that occure on antoher decision level than the
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
    fn initialze_result(
        &mut self,
        conflicting_clause: ClauseRef,
        trail: &Trail,
        levels_and_reasons: &DecisionLevelsAndReasons,
    ) -> usize {
        self.result.clear();
        // `None` stands for an undefined literal.
        self.result.push(None);
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
    fn add_resolvent(
        &mut self,
        reason: ClauseRef,
        resolve_at_lit: Option<Literal>,
        trail: &Trail,
        levels_and_reasons: &DecisionLevelsAndReasons,
    ) -> usize {
        let mut count_unresolved = 0;
        // Stamp literals on the current decision level and mark them as resolution
        // "work". All others already belong to the result: resolution is not
        // performed at these literals, since none of their inverses can appear in
        // reason clauses for variables on the current decision level. They may
        // appear in those reason clauses with the same sign, though, which is why
        // we need to keep track of the literals already included in the result.
        let current_level = trail.current_decision_level();
        if let Some(resolve_at_lit) = resolve_at_lit {
            self.stamps.unstamp(resolve_at_lit.variable());
        }
        // Optimization: To avoid pushing into the result buffer we
        // assign it to an expected capacity. In most cases this won't actually cause
        // allocations since the buffer is reused.
        let mut effective_literals_len = self.result.len();
        self.result
            .resize_with(effective_literals_len + reason.len(), || None);
        for reason_literal in reason {
            let reason_variable = reason_literal.variable();
            if Some(reason_literal) != resolve_at_lit
                && !self.stamps.is_stamped(reason_variable)
            {
                self.stamps.stamp(reason_variable);
                if levels_and_reasons.get_level(reason_variable).expect(
                    "encountered unexpected missing decision level for reason variable",
                ) == current_level
                {
                    count_unresolved += 1;
                } else {
                    self.result[effective_literals_len] = Some(reason_literal);
                    effective_literals_len += 1;
                }
            }
        }
        self.result.resize_with(effective_literals_len, || None);
        count_unresolved
    }

    /// Iteratively resolves the result buffer with reason clause of literals
    /// occurring on the current decision level, aborting when having reached the
    /// first unique implication point.
    fn resolve_until_uip(
        &mut self,
        count_unresolved: usize,
        trail: &Trail,
        levels_and_reasons: &DecisionLevelsAndReasons,
        clause_db: &ClauseDb,
    ) {
        let mut count_unresolved = count_unresolved;
        let current_level = trail.current_decision_level();
        let level_assignments = trail.level_assignments(current_level);
        let mut level_assignments = level_assignments.into_iter().rev();
        while count_unresolved != 1 {
            let resolve_at_lit = level_assignments
                .next()
                .expect("encountered unexpected missing level assignment");
            let resolve_at_var = resolve_at_lit.variable();
            let is_stamped = self.stamps.is_stamped(resolve_at_var);
            if is_stamped {
                let (level, reason) = levels_and_reasons
                    .get(resolve_at_var)
                    .expect("encountered missing reason for resolution variable");
                debug_assert_eq!(level, current_level);
                match reason {
                    None => panic!("encountered the 1-UIP too early"),
                    Some(reason) => {
                        let reason = clause_db
                            .resolve(reason)
                            .expect("error upon resolving reason clause");
                        count_unresolved += self.add_resolvent(
                            reason,
                            Some(*resolve_at_lit),
                            trail,
                            levels_and_reasons,
                        );
                        count_unresolved -= 1;
                    }
                }
            }
        }
        let asserting_literal = *level_assignments
            .find(|literal| {
                let var = literal.variable();
                self.stamps.is_stamped(var)
            })
            .expect("encountered missing asserting literal");
        self.result[0] = Some(asserting_literal);
        self.stamps.unstamp(asserting_literal.variable());
        assert_eq!(
            count_unresolved, 1,
            "reached the end of the decision level assignments without finding the 1-UIP"
        );
    }

    /// Resets the stamps for the variables of the given literals.
    fn clear_stamps(&mut self) {
        for literal in &self.result {
            let literal =
                literal.expect("encountered undefined literal after clause learning");
            self.stamps.unstamp(literal.variable());
        }
    }
}
