use bounded::{
    Bool,
    Index,
};
use core::{
    convert::TryFrom,
    num::NonZeroU32,
    ops::Not,
};

/// The sign of a literal.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Sign {
    True = 0,
    False = 1,
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
        (self as u8) == 1
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
#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct Literal {
    value: u32,
}

impl Literal {
    /// Returns the variable of the literal.
    #[inline]
    pub fn variable(self) -> Variable {
        Variable::from(self)
    }

    /// Returns `true` if the literal has positive polarity.
    #[inline]
    pub fn is_positive(self) -> bool {
        self.value & 1 == 0
    }

    /// Returns `true` if the literal has negative polarity.
    #[inline]
    pub fn is_negative(self) -> bool {
        self.value & 1 != 0
    }

    /// Returns the assignment and polarity of the literal.
    pub fn assignment(self) -> Sign {
        match self.is_positive() {
            true => Sign::True,
            false => Sign::False,
        }
    }
}

impl From<i32> for Literal {
    #[inline]
    fn from(x: i32) -> Self {
        debug_assert!(x != 0);
        let var = x.abs() as u32 - 1;
        let sign = (x < 0) as u32;
        Literal {
            value: (var << 1) + sign,
        }
    }
}

impl From<cnf_parser::Literal> for Literal {
    #[inline]
    fn from(literal: cnf_parser::Literal) -> Self {
        Self::from(literal.into_value().get())
    }
}

impl Not for Literal {
    type Output = Self;

    #[inline]
    fn not(self) -> Self::Output {
        Self {
            value: self.value ^ 1,
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
            value: NonZeroU32::new((literal.value >> 1) + 1)
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
        Literal::from(value)
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
