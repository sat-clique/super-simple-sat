use crate::{
    assignment::Assignment,
    clause_db::{
        ClauseDb,
        ClauseId,
    },
    occurrence_map::OccurrenceMap,
    Error,
};
pub use crate::{
    assignment::{
        Literal,
        Model,
        VarAssignment,
        Variable,
    },
    clause_db::Clause,
};

#[derive(Debug, PartialEq, Eq)]
enum ClauseStatus {
    Conflicting,
    UndeterminedLiteral(Literal),
    NoConflictNorForcedAssignment,
}

#[derive(Debug, PartialEq, Eq)]
pub enum PropagationResult {
    Conflict { decision: DecisionId },
    Consistent { decision: DecisionId },
}

#[derive(Debug, Default, Clone)]
pub struct Propagator {
    last_model: Option<Model>,
    propagation_queue: Vec<Literal>,
    decisions: Vec<Decision>,
    level_assignments: Vec<Literal>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct DecisionId(usize);

impl DecisionId {
    fn from_index(index: usize) -> Self {
        Self(index)
    }

    fn into_index(self) -> usize {
        self.0
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Decision {
    reason: Literal,
    start_index: usize,
    end_index: usize,
}

impl Propagator {
    pub fn last_model(&self) -> Option<&Model> {
        self.last_model.as_ref()
    }

    pub fn update_last_model(&mut self, assignment: &Assignment) -> Result<(), Error> {
        match &mut self.last_model {
            Some(last_model) => {
                last_model
                    .from_reuse(&assignment)
                    .expect("encountered unexpected incomplete assignment");
            }
            none => {
                *none = Some(
                    Model::new(&assignment)
                        .expect("encountered unexpected incomplete assignment"),
                );
            }
        }
        Ok(())
    }

    fn get_clause_status(
        &self,
        id: ClauseId,
        clauses: &ClauseDb,
        assignment: &mut Assignment,
    ) -> Option<ClauseStatus> {
        let mut num_indeterminate_lits = 0;
        let mut last_indeterminate_lit = None;
        for lit in clauses.resolve(id)? {
            match assignment
                .is_satisfied(lit)
                .expect("encountered unexpected invalid literal")
            {
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

    pub fn propagate(
        &mut self,
        root_literal: Literal,
        clauses: &ClauseDb,
        occurrence_map: &OccurrenceMap,
        assignment: &mut Assignment,
    ) -> Result<PropagationResult, Error> {
        let start = self.level_assignments.len();
        self.propagation_queue.clear();
        self.propagation_queue.push(root_literal);
        self.level_assignments.push(root_literal);
        while let Some(lit_to_propagate) = self.propagation_queue.pop() {
            for possibly_false_clause_id in
                occurrence_map.iter_potentially_conflicting_clauses(lit_to_propagate)
            {
                match self
                    .get_clause_status(possibly_false_clause_id, clauses, assignment)
                    .expect("encountered invalid clause identifier")
                {
                    ClauseStatus::Conflicting => {
                        let end = self.level_assignments.len();
                        let decision_id = DecisionId::from_index(self.decisions.len());
                        self.decisions.push(Decision {
                            start_index: start,
                            end_index: end,
                            reason: root_literal,
                        });
                        return Ok(PropagationResult::Conflict {
                            decision: decision_id,
                        })
                    }
                    ClauseStatus::UndeterminedLiteral(propagation_lit) => {
                        self.level_assignments.push(propagation_lit);
                        let (variable, var_assignment) =
                            propagation_lit.into_var_and_assignment();
                        assignment.assign(variable, var_assignment)?;
                        self.propagation_queue.push(propagation_lit);
                    }
                    _ => (),
                }
            }
        }
        let end = self.level_assignments.len();
        let decision_id = DecisionId::from_index(self.decisions.len());
        self.decisions.push(Decision {
            start_index: start,
            end_index: end,
            reason: root_literal,
        });
        Ok(PropagationResult::Consistent {
            decision: decision_id,
        })
    }

    pub fn unassign_decision(
        &mut self,
        decision: DecisionId,
        assignment: &mut Assignment,
    ) -> Result<(), Error> {
        let decision = self
            .decisions
            .get(decision.into_index())
            .ok_or_else(|| Error::InvalidDecisionId)?;
        if decision.start_index >= self.level_assignments.len() {
            return Err(Error::InvalidDecisionStart)
        }
        if decision.end_index != self.level_assignments.len() {
            return Err(Error::InvalidDecisionEnd)
        }
        for index in decision.start_index..decision.end_index {
            assignment.unassign(self.level_assignments[index].variable())?;
        }
        self.level_assignments.truncate(decision.start_index);
        Ok(())
    }
}
