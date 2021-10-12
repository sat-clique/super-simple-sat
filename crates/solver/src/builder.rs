use crate::{
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
        self.solver.consume_clause(self.current_clause.drain(..))?;
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
        self.solver.new_literal_chunk(num_variables);
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
