# error-context

A crate that adds context to Rust error `struct`s and `enum`s.
This is accomplished by means of an attribute macro, `#[contextual_error]`,
which modifies the original type to add a field that contains error context.

Specifically, the added context makes it easy to find the exact location in
the source code where an error value was emitted, if it was annotated with the
`#[contextual_error]` attribute.

## Usage

Add this to Cargo.toml:

``` toml
error-context-facade = "0.4.0"
```

Then given an error struct or enum:

``` rust
pub struct MyError {
    f0: String,
}
```

Modify the type like this:

``` rust
use error_context_facade::contextual_error;

#[contextual_error]
pub struct MyError {
    f0: String,
}
```

Supported are `struct`s and `enum`s. Unions in particular are not supported.

## Features

### Generate constructors

Given an error type, constructors will be generated for it to enable users to
avoid having to manually instantiate the error context:

``` rust
#[contextual_error]
pub struct MyStructError {
    f0: String,
}

#[contextual_error]
pub enum MyEnumError {
    Tuple(usize),
    Named { f0: String },
    Unit,
}

fn main() {
    let struct_error = MyStructError::new("I am a struct error");

    let tuple_enum_error = MyEnumError::new_Tuple(42_usize);
    let named_enum_error = MyEnumError::new_Named("I am an enum error");
    let unit_enum_error = MyEnumError::new_Unit();
}
```

### Inline constructors

Generated constructors can be inlined by using the `inline_ctors` argument:

``` rust
#[contextual_error(inline_ctors)]
pub struct MyStructError {
    f0: String,
}
```

``` rust
#[contextual_error(inline_ctors(always))]
pub struct MyStructError {
    f0: String,
}
```

``` rust
#[contextual_error(inline_ctors(never))]
pub struct MyStructError {
    f0: String,
}
```

These compile down to their regular `#[inline]`, `#[inline(always)]`
and `#[inline(never]` counterparts.

### Feature

It is possible to use a build [feature](https://doc.rust-lang.org/cargo/reference/features.html)
to control whether or not an error context is generated for a type:

``` toml
#[features]
my-error-feature = []
```

``` rust
#[contextual_error(feature = "my-error-feature")]
pub struct MyStructError {
    f0: String,
}
```

The context will only be generated if the feature is provided, either as
default feature in `Cargo.toml` or via the CLI.
