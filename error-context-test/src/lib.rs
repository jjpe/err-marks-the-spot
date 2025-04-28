//!
#![allow(unused)]

use error_context::{contextual_error, ErrorCtx};

#[contextual_error(feature = "example-build-flag", inline_ctors)]
#[derive(Debug)]
pub struct TupleStructError(usize, String);

#[contextual_error(feature = "example-build-flag", inline_ctors(always))]
#[derive(Debug)]
pub struct NamedStructError {
    f0: String,
}

#[contextual_error(feature = "example-build-flag", inline_ctors(never))]
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
    fn add_error_ctx_field() {
        let tuple_strct_error = TupleStructError(
            0,
            "blah".to_string(),
            error_context::ErrorCtx::new(),
        );
        let named_strct_error = NamedStructError {
            f0: "foo".to_string(),
            ctx: error_context::ErrorCtx::new(),
        };
        let unit_strct_error = UnitStructError {
            ctx: error_context::ErrorCtx::new(),
        };

        let tuple_enum_error = EnumError::Tuple(
            300,
            error_context::ErrorCtx::new(),
        );
        let named_enum_error = EnumError::Named {
            f0: 42,
            ctx: error_context::ErrorCtx::new(),
        };
        let unit_enum_error = EnumError::Unit {
            ctx: error_context::ErrorCtx::new(),
        };
    }

    #[test]
    fn use_ctors() {
        let tuple_strct_error = TupleStructError::new(
            0_usize,
            "blah".to_string(),
        );
        let named_strct_error = NamedStructError::new("foo".to_string());
        let unit_strct_error = UnitStructError::new();

        let tuple_enum_error = EnumError::new_Tuple(300_usize);
        let named_enum_error = EnumError::new_Named(42_usize);
        let unit_enum_error = EnumError::new_Unit();
    }
}
