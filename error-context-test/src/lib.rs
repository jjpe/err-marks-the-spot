//!
#![allow(unused)]

use error_context_macro::contextual_error;

#[contextual_error(feature = "example-build-flag")]
#[derive(Debug)]
pub struct TupleStructError(usize, String);

#[contextual_error(feature = "example-build-flag")]
#[derive(Debug)]
pub struct NamedStructError {
    f0: String,
}

#[contextual_error(feature = "example-build-flag")]
#[derive(Debug)]
pub struct UnitStructError;

#[contextual_error(feature = "example-build-flag")]
#[derive(Debug)]
pub enum EnumError {
    Tuple(usize),
    Named { f0: usize, },
    Unit,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_location_and_backtrace() {
        let tuple_strct_error = TupleStructError(
            0,
            "blah".to_string(),
            std::panic::Location::caller(),
            std::backtrace::Backtrace::capture(),
        );
        let named_strct_error = NamedStructError {
            f0: "foo".to_string(),
            location: std::panic::Location::caller(),
            backtrace: std::backtrace::Backtrace::capture(),
        };
        let unit_strct_error = UnitStructError {
            location: std::panic::Location::caller(),
            backtrace: std::backtrace::Backtrace::capture(),
        };

        let tuple_enum_error = EnumError::Tuple(
            300,
            std::panic::Location::caller(),
            std::backtrace::Backtrace::capture(),
        );
        let named_enum_error = EnumError::Named {
            f0: 42,
            location: std::panic::Location::caller(),
            backtrace: std::backtrace::Backtrace::capture(),
        };
        let unit_enum_error = EnumError::Unit {
            location: std::panic::Location::caller(),
            backtrace: std::backtrace::Backtrace::capture(),
        };
    }

    #[test]
    fn add_constructors() {
        let tuple_strct_error = TupleStructError::new(
            0,
            "blah".to_string(),
        );
        let named_strct_error = NamedStructError::new("foo".to_string());
        let unit_strct_error = UnitStructError::new();

        let tuple_enum_error = EnumError::new_Tuple(300);
        let named_enum_error = EnumError::new_Named(42);
        let unit_enum_error = EnumError::new_Unit();
    }
}
