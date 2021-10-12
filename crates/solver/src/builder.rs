use crate::{
    ClauseSanitizer,
    Error,
    Literal,
    SanitizedLiterals,
    Solver,
};
use cnf_parser::Output;

#[derive(Debug, Default)]
pub struct SolverBuilder {
    solver: Solver,
    num_variables: Option<usize>,
    current_clause: Vec<Literal>,
    sanitizer: ClauseSanitizer,
}

impl SolverBuilder {
    fn finalize_current_clause(&mut self) -> Result<(), <Self as Output>::Error> {
        if self.num_variables.is_none() {
            return Err("missing problem line before clause inputs".into())
        }
        match self.sanitizer.sanitize(self.current_clause.drain(..)) {
            SanitizedLiterals::Literals(literals) => {
                self.solver.consume_clause(literals)?;
            }
            SanitizedLiterals::UnitClause(unit) => {
                self.solver.enqueue_assumption(unit)?;
            }
            SanitizedLiterals::TautologicalClause => (),
            SanitizedLiterals::EmptyClause => {
                return Err("encountered empty or self conflicting clause".into())
            }
        }
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
