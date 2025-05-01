//!
#![allow(unused)]

use err_marks_the_spot::{ErrorCtx, err_marks_the_spot};

/// FOo qux baz {0} {1}
///   - BaR Quux {0}.
#[err_marks_the_spot(feature = "example-build-flag", inline_ctors)]
#[derive(Debug)]
pub struct TupleStructError(usize, String);

/// This error contains a field f0={f0}
#[err_marks_the_spot(feature = "example-build-flag", inline_ctors(never))]
#[derive(Debug)]
pub struct NamedStructError {
    f0: String,
}

/// This error contains no fields
#[err_marks_the_spot(feature = "example-build-flag", inline_ctors(never))]
#[derive(Debug)]
pub struct UnitStructError;

/// enum-level docstring: EnumError has 3 variants
#[err_marks_the_spot(feature = "example-build-flag")]
#[derive(Debug)]
pub enum EnumError {
    /// This is a tuple variant: {0}, {2} and {1}
    /// And another thing, tuple variants are like tuple structs.
    Tuple(usize, String, bool),
    /// This is a named variant: {f0} and {f1}
    /// And another thing, named variants are like named structs.
    Named { f0: u8, f1: String },
    /// This is a unit variant.
    /// And another thing, unit variants are like unit structs.
    Unit,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_error_ctx_field() {
        let tuple_struct_error = TupleStructError(
            17,
            "blah".to_string(),
            #[cfg(feature = "example-build-flag")]
            err_marks_the_spot::ErrorCtx::new(),
        );
        println!("{tuple_struct_error}");
        let named_struct_error = NamedStructError {
            f0: "happy little field".to_string(),
            #[cfg(feature = "example-build-flag")]
            ctx: err_marks_the_spot::ErrorCtx::new(),
        };
        println!("{named_struct_error}");
        let unit_struct_error = UnitStructError {
            #[cfg(feature = "example-build-flag")]
            ctx: err_marks_the_spot::ErrorCtx::new(),
        };
        println!("{unit_struct_error}");

        let tuple_enum_error = EnumError::Tuple(
            300,
            "tuple variant".to_string(),
            false,
            #[cfg(feature = "example-build-flag")]
            err_marks_the_spot::ErrorCtx::new(),
        );
        let named_enum_error = EnumError::Named {
            f0: 42,
            f1: "named variant".to_string(),
            #[cfg(feature = "example-build-flag")]
            ctx: err_marks_the_spot::ErrorCtx::new(),
        };
        let unit_enum_error = EnumError::Unit {
            #[cfg(feature = "example-build-flag")]
            ctx: err_marks_the_spot::ErrorCtx::new(),
        };
    }

    #[test]
    fn use_ctors() {
        let tuple_struct_error = TupleStructError::new(24_usize, "blahs");
        println!("{tuple_struct_error}");
        let named_struct_error = NamedStructError::new("foo");
        println!("{named_struct_error}");
        let unit_struct_error = UnitStructError::new();
        println!("{unit_struct_error}");

        #[rustfmt::skip]
        let tuple_enum_error = EnumError::new_Tuple(
            300_usize,
            "tuple variant".to_string(),
            false,
        );
        println!("{tuple_enum_error}");
        #[rustfmt::skip]
        let named_enum_error = EnumError::new_Named(
            42_u8,
            "named variant".to_string(),
        );
        println!("{named_enum_error}");
        let unit_enum_error = EnumError::new_Unit();
        println!("{unit_enum_error}");
    }
}
