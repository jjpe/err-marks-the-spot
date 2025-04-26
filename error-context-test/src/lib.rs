//!

use error_context_macro::contextual_error;

#[contextual_error]
pub struct FooError {
    pub field0: usize,
}



#[cfg(test)]
mod tests {

    #[test]
    fn it_works() {
        println!("blah blah blah");
    }
}
