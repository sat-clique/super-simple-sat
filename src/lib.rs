mod assignment;
mod builder;
mod clause_db;
mod occurrence_map;

#[cfg(test)]
mod tests;

use crate::{
    assignment::{
        Assignment,
        Literal,
        VarAssignment,
        Variable,
    },
    builder::SolverBuilder,
    clause_db::{
        Clause,
        ClauseDb,
        ClauseId,
    },
    occurrence_map::OccurrenceMap,
};
use cnf_parser::{
    Error as CnfError,
    Input,
};

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Other(&'static str),
}

impl From<&'static str> for Error {
    fn from(message: &'static str) -> Self {
        Self::Other(message)
    }
}

#[derive(Debug, PartialEq, Eq)]
enum ClauseStatus {
    Conflicting,
    UndeterminedLiteral(Literal),
    NoConflictNorForcedAssignment,
}

#[derive(Debug, PartialEq, Eq)]
enum PropagationResult {
    Conflict { assigned: Vec<Literal> },
    Consistent { assigned: Vec<Literal> },
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum SolveResult {
    Conflict,
    Sat,
}

#[derive(Debug, Default, Clone)]
pub struct Solver {
    clauses: ClauseDb,
    occurrence_map: OccurrenceMap,
    assignments: Assignment,
    last_model: Option<Assignment>,
}

impl Solver {
    pub fn from_cnf<I>(input: &mut I) -> Result<Self, CnfError<Error>>
    where
        I: Input,
    {
        let mut builder = SolverBuilder::default();
        cnf_parser::parse_cnf(input, &mut builder)?;
        Ok(builder.finalize())
    }

    pub fn consume_clause(&mut self, clause: Clause) {
        let id = self.clauses.push(clause);
        for literal in self
            .clauses
            .resolve(id)
            .expect("unexpected missing clause that has just been inserted")
        {
            self.occurrence_map.register_for_lit(literal, id)
        }
    }

    pub fn new_literal(&mut self) -> Literal {
        self.assignments
            .new_variable()
            .into_literal(VarAssignment::True)
    }

    fn get_clause_status(&self, id: ClauseId) -> Option<ClauseStatus> {
        let mut num_indeterminate_lits = 0;
        let mut last_indeterminate_lit = None;
        for lit in self.clauses.resolve(id)? {
            match self.assignments.is_satisfied(lit) {
                Some(true) => return Some(ClauseStatus::NoConflictNorForcedAssignment),
                Some(false) => {}
                None => {
                    last_indeterminate_lit = Some(lit);
                    num_indeterminate_lits += 1;
                }
            }
        }
        match num_indeterminate_lits {
            0 => Some(ClauseStatus::Conflicting),
            1 => {
                Some(ClauseStatus::UndeterminedLiteral(
                    last_indeterminate_lit
                        .expect("encountered missing expected undetermined literal"),
                ))
            }
            _ => Some(ClauseStatus::NoConflictNorForcedAssignment),
        }
    }

    fn propagate(&mut self, root_literal: Literal) -> PropagationResult {
        let mut propagation_queue = vec![root_literal];
        let mut level_assignments = vec![root_literal];
        while let Some(lit_to_propagate) = propagation_queue.pop() {
            for possibly_false_clause_id in self
                .occurrence_map
                .iter_potentially_conflicting_clauses(lit_to_propagate)
            {
                match self
                    .get_clause_status(possibly_false_clause_id)
                    .expect("encountered invalid clause identifier")
                {
                    ClauseStatus::Conflicting => {
                        return PropagationResult::Conflict {
                            assigned: level_assignments,
                        }
                    }
                    ClauseStatus::UndeterminedLiteral(propagation_lit) => {
                        level_assignments.push(propagation_lit);
                        let (variable, var_assignment) =
                            propagation_lit.into_var_and_assignment();
                        self.assignments.assign(variable, var_assignment);
                        propagation_queue.push(propagation_lit);
                    }
                    _ => (),
                }
            }
        }
        PropagationResult::Consistent {
            assigned: level_assignments,
        }
    }

    fn undo_current_level_assignments<L>(&mut self, conflicting_lits: L)
    where
        L: IntoIterator<Item = Literal>,
    {
        for conflicting_lit in conflicting_lits {
            self.assignments.unassign(conflicting_lit.variable());
        }
    }

    fn solve_for(
        &mut self,
        current_var: Variable,
        assignment: VarAssignment,
    ) -> SolveResult {
        self.assignments.assign(current_var, assignment);
        let current_lit = current_var.into_literal(assignment);
        match self.propagate(current_lit) {
            PropagationResult::Conflict { assigned } => {
                self.undo_current_level_assignments(assigned);
                SolveResult::Conflict
            }
            PropagationResult::Consistent { assigned } => {
                let next_var = self.assignments.next_unassigned(Some(current_var));
                let result = match next_var {
                    None => {
                        self.last_model = Some(self.assignments.clone());
                        SolveResult::Sat
                    }
                    Some(unassigned_var) => {
                        if let SolveResult::Sat =
                            self.solve_for(unassigned_var, VarAssignment::True)
                        {
                            SolveResult::Sat
                        } else if let SolveResult::Sat =
                            self.solve_for(unassigned_var, VarAssignment::False)
                        {
                            SolveResult::Sat
                        } else {
                            SolveResult::Conflict
                        }
                    }
                };
                self.undo_current_level_assignments(assigned);
                result
            }
        }
    }

    pub fn solve<L>(&mut self, assumptions: L) -> bool
    where
        L: IntoIterator<Item = Literal>,
    {
        // If the set of clauses contain the empty clause: UNSAT
        if self.assignments.len_variables() == 0 {
            return true
        }
        for assumption in assumptions {
            let (variable, assignment) = assumption.into_var_and_assignment();
            self.assignments.assign(variable, assignment);
            if let PropagationResult::Conflict { assigned: _ } =
                self.propagate(assumption)
            {
                return false
            }
        }
        let initial_var = self.assignments.next_unassigned(None);
        match initial_var {
            None => return true,
            Some(initial_var) => {
                if let SolveResult::Sat = self.solve_for(initial_var, VarAssignment::True)
                {
                    return true
                }
                if let SolveResult::Sat =
                    self.solve_for(initial_var, VarAssignment::False)
                {
                    return true
                }
                false
            }
        }
    }

    pub fn last_model(&self) -> Option<&Assignment> {
        self.last_model.as_ref()
    }

    pub fn print_last_model(&self) {
        if let Some(last_model) = &self.last_model {
            for (variable, assignment) in last_model {
                let index = variable.into_index();
                let assignment = match assignment {
                    Some(assigned) => assigned.to_bool().to_string(),
                    None => "unassigned".to_string(),
                };
                println!("Var: {:3}\t Value: {}", index, assignment);
            }
        }
    }
}
