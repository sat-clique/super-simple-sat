use crate::{
    clause_db::Clause,
    Error,
    Literal,
    Solver,
    Variable,
};
use cnf_parser::Output;

#[derive(Debug, Default)]
pub struct SolverBuilder {
    solver: Solver,
    num_variables: Option<usize>,
    max_seen_variable: Option<Variable>,
    current_clause: Vec<Literal>,
}

impl SolverBuilder {
    fn finalize_current_clause(&mut self) -> Result<(), <Self as Output>::Error> {
        let accumulated_lits = core::mem::take(&mut self.current_clause);
        let clause = Clause::new(accumulated_lits)
            .map_err(|_| "encountered empty or self conflicting clause")?;
        self.solver.consume_clause(clause);
        Ok(())
    }

    pub fn finalize(self) -> Solver {
        self.solver
    }
}

impl Output for SolverBuilder {
    type Error = Error;

    fn problem(
        &mut self,
        num_variables: u32,
        _num_clauses: u32,
    ) -> Result<(), Self::Error> {
        let num_variables = num_variables as usize;
        self.num_variables = Some(num_variables);
        self.solver
            .assignments
            .new_chunk_of_variables(num_variables as usize)
            .map_err(|_| Error::Other("allocated too many variables"))?;
        Ok(())
    }

    fn literal(&mut self, literal: cnf_parser::Literal) -> Result<(), Self::Error> {
        let literal = literal.into();
        self.current_clause.push(literal);
        match &mut self.max_seen_variable {
            Some(max_seen) => {
                let variable = literal.variable();
                if variable.into_index() > max_seen.into_index() {
                    *max_seen = variable;
                }
            }
            none => {
                *none = Some(literal.variable());
            }
        }
        Ok(())
    }

    fn finalize_clause(&mut self) -> Result<(), Self::Error> {
        self.finalize_current_clause()?;
        Ok(())
    }

    fn finish(&mut self) -> Result<(), Self::Error> {
        if !self.current_clause.is_empty() {
            self.finalize_current_clause()?;
        }
        if self.num_variables.is_none() {
            let amount = self
                .max_seen_variable
                .map(|variable| variable.into_index() + 1)
                .unwrap_or_else(|| 0);
            self.solver
                .assignments
                .new_chunk_of_variables(amount)
                .map_err(|_| Error::Other("allocated too many variables"))?;
        }
        Ok(())
    }
}
