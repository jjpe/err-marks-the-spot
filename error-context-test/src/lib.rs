//!

use error_context_macro::contextual_error;

#[contextual_error]
#[derive(Debug)]
pub struct FooError(usize);

// TODO: Generate a ctor for the type on which `#[contextual_error]` is used,
//       because otherwise the Location and Backtrace need to be provided
//       manually, which is super annoying.
//       For structs this can be a single ctor called `new`, while for enums it
//       should be a single ctor per enum variant carrying the variant's name.


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let err = FooError(
            0,
            std::panic::Location::caller(),
            std::backtrace::Backtrace::capture(),
        );
    }
}
