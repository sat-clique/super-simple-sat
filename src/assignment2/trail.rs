use crate::{
    utils::{
        BoundedStack,
        Index,
    },
    Error,
    Literal,
};

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TrailLimit(usize);

impl Index for TrailLimit {
    fn from_index(index: usize) -> Self {
        Self(index)
    }

    fn into_index(self) -> usize {
        self.0
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DecisionLevel(usize);

impl Index for DecisionLevel {
    fn from_index(index: usize) -> Self {
        Self(index)
    }

    fn into_index(self) -> usize {
        self.0
    }
}

#[derive(Debug)]
pub struct TrailLimits {
    limits: Vec<TrailLimit>,
}

impl Default for TrailLimits {
    fn default() -> Self {
        Self {
            limits: vec![TrailLimit(0)],
        }
    }
}

impl TrailLimits {
    /// Pushes a new limit to the trail limits.
    pub fn push(&mut self, new_limit: TrailLimit) -> DecisionLevel {
        let index = self.limits.len();
        self.limits.push(new_limit);
        DecisionLevel::from_index(index)
    }

    /// Returns the last trail limit.
    pub fn last(&self) -> TrailLimit {
        *self
            .limits
            .last()
            .expect("encountered unexpected empty trail limits")
    }

    /// Pops the trail limits to the given decision level.
    pub fn pop_to_level(&mut self, level: DecisionLevel) -> TrailLimit {
        assert!(level.into_index() >= 1);
        assert!(level.into_index() < self.limits.len());
        self.limits.truncate(level.into_index() + 1);
        self.last()
    }
}

#[derive(Debug, Default)]
pub struct Trail {
    decisions: BoundedStack<Literal>,
    limits: TrailLimits,
}

impl Trail {
    /// Returns the current number of variables.
    fn len_variables(&self) -> usize {
        self.decisions.capacity()
    }

    /// Registers the given number of additional variables.
    ///
    /// # Errors
    ///
    /// If the number of total variables is out of supported bounds.
    pub fn register_new_variables(&mut self, new_variables: usize) -> Result<(), Error> {
        let total_variables = self.len_variables() + new_variables;
        self.decisions.increase_capacity_to(total_variables);
        Ok(())
    }

    /// Pushes a new decision level and returns it.
    pub fn new_decision_level(&mut self) -> DecisionLevel {
        let limit = TrailLimit::from_index(self.decisions.len());
        let index = self.limits.push(limit);
        index
    }

    /// Backjumps the trail to the given decision level.
    pub fn pop_to_level<F>(&mut self, level: DecisionLevel, mut observer: F)
    where
        F: FnMut(Literal),
    {
        let limit = self.limits.pop_to_level(level);
        self.decisions
            .pop_to(limit.into_index(), |popped| observer(*popped))
            .expect("encountered unexpected invalid trail limit");
    }
}
