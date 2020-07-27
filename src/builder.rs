use crate::{
    clause_db::Clause,
    Error,
    Literal,
    Solver,
};
use cnf_parser::Output;

#[derive(Debug, Default)]
pub struct SolverBuilder {
    solver: Solver,
    num_variables: Option<usize>,
    current_clause: Vec<Literal>,
}

impl SolverBuilder {
    fn finalize_current_clause(&mut self) -> Result<(), <Self as Output>::Error> {
        if self.num_variables.is_none() {
            return Err("missing problem line before clause inputs".into())
        }
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
        Ok(())
    }
}
