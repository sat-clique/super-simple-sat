use core::{
    convert::TryFrom,
    num::{
        NonZeroI32,
        NonZeroU32,
    },
    ops::Not,
};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum VarAssignment {
    True,
    False,
}

impl VarAssignment {
    pub fn to_bool(self) -> bool {
        match self {
            Self::True => true,
            Self::False => false,
        }
    }
}

/// A literal of a variable with its polarity.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Literal {
    value: NonZeroI32,
}

impl Literal {
    /// Returns the variable of the literal.
    pub fn variable(self) -> Variable {
        Variable::from(self)
    }

    /// Returns `true` if the literal has negative polarity.
    pub fn is_negative(self) -> bool {
        self.value.get().is_negative()
    }

    /// Returns `true` if the literal has positive polarity.
    pub fn is_positive(self) -> bool {
        self.value.get().is_positive()
    }

    /// Returns the literal's variable and polarity.
    pub fn into_var_and_assignment(self) -> (Variable, VarAssignment) {
        (
            self.variable(),
            match self.is_positive() {
                true => VarAssignment::True,
                false => VarAssignment::False,
            },
        )
    }
}

impl Not for Literal {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self {
            value: NonZeroI32::new(-self.value.get())
                .expect("encountered zero i32 from non-zero i32"),
        }
    }
}

impl From<cnf_parser::Literal> for Literal {
    fn from(literal: cnf_parser::Literal) -> Self {
        Self {
            value: literal.into_value(),
        }
    }
}

/// A unique variable.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Variable {
    value: NonZeroU32,
}

impl From<Literal> for Variable {
    fn from(literal: Literal) -> Self {
        Self {
            value: NonZeroU32::new(literal.value.get().abs() as u32)
                .expect("encountered unexpected zero i32"),
        }
    }
}

impl Variable {
    /// Returns the variable for the given index if valid.
    ///
    /// # Note
    ///
    /// This solver only supports up to 2^31-1 unique variables.
    /// Any index that is out of this range is invalid for this operation.
    pub fn from_index(index: usize) -> Option<Self> {
        let index = i32::try_from(index).ok()?;
        NonZeroU32::new((index as u32).wrapping_add(1)).map(|shifted_index| {
            Self {
                value: shifted_index,
            }
        })
    }

    /// Returns the literal for the variable with the given polarity.
    pub fn into_literal(self, assignment: VarAssignment) -> Literal {
        let value = match assignment {
            VarAssignment::True => self.value.get() as i32,
            VarAssignment::False => -1 * self.value.get() as i32,
        };
        Literal {
            value: NonZeroI32::new(value).expect("encountered unexpected zero i32"),
        }
    }

    /// Returns the index of the variable.
    pub fn into_index(self) -> usize {
        self.value.get() as usize - 1
    }
}
