use crate::Literal;
use alloc::vec::Vec;
use hashbrown::HashSet;

/// Verifies and builds up clauses from the incoming stream of literals.
///
/// The purpose of this type is to cache and reuse buffers
/// that are needed for the verification of the incoming clause
/// literals.
#[derive(Debug, Default, Clone)]
pub struct ClauseBuilder {
    literals: Vec<Literal>,
    occurrences: HashSet<Literal>,
}

/// Errors that can occure upon building or verifying clauses.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Error {
    /// The clause contain no literals.
    EmptyClause,
    /// The clause is a tautology and can be ignored by the solver.
    ///
    /// This happens whenever a clause contains the same literal twice
    /// but with different polarities.
    TautologicClause,
    /// The clause is a unit clause with exactly one literal.
    UnitClause { literal: Literal },
}

/// A verified clause and its literals.
///
/// The clause if verified to ...
/// - have at least 2 literals
/// - not be self contradicting
/// - not have duplicate literals
///
/// # Note
///
/// Can be used to store into the clause database.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct VerifiedClause<'a> {
    pub literals: &'a [Literal],
}

impl ClauseBuilder {
    /// Builds a new clause from the given literals.
    ///
    /// # Note
    ///
    /// The built clause can be use to store into the clause database.
    ///
    /// # Errors
    ///
    /// - If the literal sequence is empty.
    /// - If the literal sequence represents a unit clause.
    ///   In this case the unit literal is returned as error to allow for
    ///   further error handling.
    /// - If the literal sequence contradicts itself, e.g. contains `a AND !a`
    ///   where `a` is a literal and `!a` its negation.
    pub fn build<L>(&mut self, clause_literals: L) -> Result<VerifiedClause, Error>
    where
        L: IntoIterator<Item = Literal>,
    {
        let Self {
            literals,
            occurrences,
        } = self;
        literals.clear();
        literals.extend(clause_literals);
        if literals.is_empty() {
            return Err(Error::EmptyClause)
        }
        literals.sort_unstable();
        literals.dedup();
        if literals.len() == 1 {
            return Err(Error::UnitClause {
                literal: literals[0],
            })
        }
        occurrences.clear();
        for &literal in literals.iter() {
            if occurrences.contains(&!literal) {
                return Err(Error::TautologicClause)
            }
            occurrences.insert(literal);
        }
        Ok(VerifiedClause { literals })
    }
}
