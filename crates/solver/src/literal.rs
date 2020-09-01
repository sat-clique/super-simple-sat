use bounded::{
    Bool,
    Index,
};
use core::{
    convert::TryFrom,
    fmt::{
        Display,
        Formatter,
        Result,
    },
    num::{
        NonZeroI32,
        NonZeroU32,
    },
    ops::Not,
};

/// The sign of a literal.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Sign {
    True,
    False,
}

impl Bool for Sign {
    /// Creates a sign from the given `bool` value.
    ///
    /// - `false` becomes `Sign::False`
    /// - `true` becomes `Sign::True`
    fn from_bool(value: bool) -> Self {
        match value {
            true => Self::True,
            false => Self::False,
        }
    }

    /// Converts the sign into a `bool` value.
    ///
    /// - `Sign::True` becomes `true`
    /// - `Sign::False` becomes `false`
    fn into_bool(self) -> bool {
        match self {
            Self::True => true,
            Self::False => false,
        }
    }
}

impl Not for Sign {
    type Output = Self;

    fn not(self) -> Self::Output {
        match self {
            Self::True => Self::False,
            Self::False => Self::True,
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
    #[inline]
    pub fn variable(self) -> Variable {
        Variable::from(self)
    }

    /// Returns `true` if the literal has negative polarity.
    #[inline]
    pub fn is_negative(self) -> bool {
        self.value.get().is_negative()
    }

    /// Returns `true` if the literal has positive polarity.
    #[inline]
    pub fn is_positive(self) -> bool {
        self.value.get().is_positive()
    }

    /// Returns the assignment and polarity of the literal.
    pub fn assignment(self) -> Sign {
        match self.is_positive() {
            true => Sign::True,
            false => Sign::False,
        }
    }
}

impl Display for Literal {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        self.value.get().fmt(f)
    }
}

impl Not for Literal {
    type Output = Self;

    #[inline]
    fn not(self) -> Self::Output {
        Self {
            value: NonZeroI32::new(-self.value.get())
                .expect("encountered zero i32 from non-zero i32"),
        }
    }
}

impl From<cnf_parser::Literal> for Literal {
    #[inline]
    fn from(literal: cnf_parser::Literal) -> Self {
        Self {
            value: literal.into_value(),
        }
    }
}

/// A unique variable.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Variable {
    value: NonZeroU32,
}

impl From<Literal> for Variable {
    #[inline]
    fn from(literal: Literal) -> Self {
        Self {
            value: NonZeroU32::new(literal.value.get().abs() as u32)
                .expect("encountered unexpected zero i32"),
        }
    }
}

impl Variable {
    /// The maximum supported number of unique variables.
    pub const MAX_LEN: usize = (u32::MAX - 1) as usize;

    /// Returns `true` if the given index is a valid variable index.
    #[inline]
    pub(crate) fn is_valid_index(index: usize) -> bool {
        i32::try_from(index).is_ok()
    }

    /// Returns the variable for the given index if valid.
    ///
    /// # Note
    ///
    /// This solver only supports up to 2^31-1 unique variables.
    /// Any index that is out of this range is invalid for this operation.
    pub(crate) fn from_index(index: usize) -> Option<Self> {
        let index = i32::try_from(index).ok()?;
        NonZeroU32::new((index as u32).wrapping_add(1)).map(|shifted_index| {
            Self {
                value: shifted_index,
            }
        })
    }

    /// Returns the literal for the variable with the given polarity.
    pub fn into_literal(self, assignment: Sign) -> Literal {
        let value = match assignment {
            Sign::True => self.value.get() as i32,
            Sign::False => -(self.value.get() as i32),
        };
        Literal {
            value: NonZeroI32::new(value).expect("encountered unexpected zero i32"),
        }
    }

    /// Returns the index of the variable.
    #[inline]
    pub(crate) fn into_index(self) -> usize {
        self.value.get() as usize - 1
    }
}

impl Index for Variable {
    fn from_index(index: usize) -> Self {
        Variable::from_index(index).expect("encountered invalid index")
    }

    fn into_index(self) -> usize {
        self.into_index()
    }
}
