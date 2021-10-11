use bounded::{
    Bool,
    Index,
};
use core::{
    convert::TryFrom,
    fmt,
    fmt::{
        Debug,
        Display,
        Formatter,
    },
    ops::Not,
};

/// The sign of a literal.
#[derive(Copy, Clone, PartialEq, Eq)]
#[repr(transparent)]
pub struct Sign(bool);

impl Debug for Sign {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self.into_bool() {
            true => write!(f, "Sign::POS"),
            false => write!(f, "Sign::NEG"),
        }
    }
}

impl Display for Sign {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        if self.is_neg() {
            write!(f, "-")?;
        }
        Ok(())
    }
}

impl Sign {
    /// The positive sign.
    pub const POS: Self = Self(true);

    /// The negative sign.
    pub const NEG: Self = Self(false);

    /// Returns `true` if the sign has positive polarity.
    pub fn is_pos(self) -> bool {
        self.0
    }

    /// Returns `true` if the sign has negative polarity.
    pub fn is_neg(self) -> bool {
        !self.is_pos()
    }

    /// Returns `1` if the sign has positive polarity and otherwise `0`.
    pub fn into_u8(self) -> u8 {
        self.0 as u8
    }
}

impl Bool for Sign {
    /// Creates a sign from the given `bool` value.
    ///
    /// - `false` becomes `Sign::NEG`
    /// - `true` becomes `Sign::POS`
    #[inline]
    fn from_bool(value: bool) -> Self {
        Self(value)
    }

    /// Converts the sign into a `bool` value.
    ///
    /// - `Sign::POS` becomes `true`
    /// - `Sign::NEG` becomes `false`
    #[inline]
    fn into_bool(self) -> bool {
        self.0
    }
}

impl Not for Sign {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self(!self.0)
    }
}

/// A literal of a variable with its polarity.
#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
#[repr(transparent)]
pub struct Literal {
    value: u32,
}

impl Literal {
    /// Returns the variable of the literal.
    #[inline]
    pub fn variable(self) -> Variable {
        Variable::from(self)
    }

    /// Returns the assignment and polarity of the literal.
    #[inline]
    pub fn sign(self) -> Sign {
        Sign((self.value & 1) == 0)
    }

    /// Flips the polarity of the literal sign.
    #[inline]
    pub fn negate(&mut self) {
        self.value ^= 1;
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
    value: u32,
}

impl From<u32> for Variable {
    fn from(value: u32) -> Self {
        Self { value }
    }
}

impl From<Literal> for Variable {
    #[inline]
    fn from(literal: Literal) -> Self {
        Self {
            value: literal.value >> 1,
        }
    }
}

impl Variable {
    /// The maximum supported number of unique variables.
    pub const MAX_LEN: usize = (u32::MAX >> 1) as usize;

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
        assert!(index < Self::MAX_LEN);
        u32::try_from(index).ok().map(|value| Self { value })
    }

    /// Returns the literal for the variable with the given polarity.
    pub fn into_literal(self, sign: Sign) -> Literal {
        let sign = sign.into_bool() as u8;
        let value = (self.value << 1) + sign as u32;
        Literal { value }
    }

    /// Returns the index of the variable.
    #[inline]
    pub(crate) fn into_index(self) -> usize {
        self.value as usize
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
