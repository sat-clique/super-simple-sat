use bounded::{
    Bool,
    Index,
};
use core::{
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
    pub const fn is_pos(self) -> bool {
        self.0
    }

    /// Returns `true` if the sign has negative polarity.
    pub const fn is_neg(self) -> bool {
        !self.is_pos()
    }

    /// Returns `1` if the sign has positive polarity and otherwise `0`.
    pub const fn into_u8(self) -> u8 {
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

    #[inline]
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
    /// Creates a new literal form the given variable and sign.
    ///
    /// # Note
    ///
    /// This is an internal API. Users should create new literals
    /// through the API provided by the solver instance.
    #[inline]
    pub(crate) fn new(var: Variable, sign: Sign) -> Self {
        let sign = sign.into_u8();
        let value = (var.value << 1) + sign as u32;
        Self { value }
    }

    /// Returns the variable of the literal.
    #[inline]
    pub fn variable(self) -> Variable {
        Variable::from(self)
    }

    /// Returns the assignment and polarity of the literal.
    #[inline]
    pub fn sign(self) -> Sign {
        Sign((self.value & 1) != 0)
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

impl Display for Literal {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.sign(), self.variable())
    }
}

/// A variable of the solver.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
    /// The maximum valid value of a variable.
    ///
    /// # Note
    ///
    /// Due to memory layout of literals and variables this solver
    /// only supports up to 2^31 unique variables.
    pub const MAX_VALUE: u32 = u32::MAX >> 1;

    /// The maximum supported variable index.
    pub const MAX_INDEX: usize = Self::MAX_VALUE as usize;

    /// Returns `true` if the given index is a valid variable index.
    #[inline]
    pub(crate) const fn is_valid_index(index: usize) -> bool {
        index <= Self::MAX_INDEX
    }

    /// Returns back the index as `u32` if it is a valid value for a literal.
    const fn filter_valid_index(index: usize) -> Option<u32> {
        if Self::is_valid_index(index) {
            return Some(index as u32)
        }
        None
    }
}

impl Index for Variable {
    #[inline]
    fn from_index(index: usize) -> Self {
        Self::filter_valid_index(index)
            .map(|value| Self { value })
            .unwrap_or_else(|| panic!("encountered invalid index: {}", index))
    }

    #[inline]
    fn into_index(self) -> usize {
        self.value as usize
    }
}

impl Display for Variable {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.into_index() + 1)
    }
}

/// Trait implemented by solver facilities that need to be informed about additional variables.
pub trait RegisterVariables {
    /// Informs the implementer about an amount of new variables for the solver to use.
    ///
    /// # Panics
    ///
    /// If the implementer cannot afford to registered the given amount of new variables.
    fn register_variables(&mut self, additional: usize);
}

#[cfg(test)]
mod tests {
    use super::*;

    mod sign {
        use super::*;

        #[test]
        fn not_works() {
            assert_eq!(!Sign::POS, Sign::NEG);
            assert_eq!(!Sign::NEG, Sign::POS);
            assert_eq!(!!Sign::POS, Sign::POS);
            assert_eq!(!!Sign::NEG, Sign::NEG);
        }

        #[test]
        fn into_u8_works() {
            assert_eq!(Sign::POS.into_u8(), 1);
            assert_eq!(Sign::NEG.into_u8(), 0);
        }

        #[test]
        fn is_pos_works() {
            assert_eq!(Sign::POS.is_pos(), true);
            assert_eq!(Sign::NEG.is_pos(), false);
        }

        #[test]
        fn is_neg_works() {
            assert_eq!(Sign::POS.is_neg(), false);
            assert_eq!(Sign::NEG.is_neg(), true);
        }
    }

    mod literal {
        use super::*;

        #[test]
        fn sign_works() {
            let v = Variable::from_index(0);
            assert_eq!(Literal::new(v, Sign::POS).sign(), Sign::POS);
            assert_eq!(Literal::new(v, Sign::NEG).sign(), Sign::NEG);
        }

        #[test]
        fn not_works() {
            let l1 = Literal::new(Variable::from_index(0), Sign::POS);
            let l2 = !l1;
            assert_ne!(l1.sign(), l2.sign());
            let l3 = !l2;
            assert_eq!(l1.sign(), l3.sign());
        }

        #[test]
        fn variable_works() {
            let v = Variable::from_index(42);
            assert_eq!(Literal::new(v, Sign::POS).variable(), v);
            assert_eq!(Literal::new(v, Sign::NEG).variable(), v);
        }
    }
}
