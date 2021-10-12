mod clause_ref;
mod db;

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    EmptyClause,
}

pub use self::{
    clause_ref::{
        ClauseRef,
        ClauseRefMut,
        PropagationResult,
    },
    db::{
        ClauseDb,
        ClauseDbIter,
        ClauseId,
    },
};
