//!

use proc_macro::{TokenStream, Span};
use proc_macro2::{Ident as Ident2, TokenStream as TokenStream2, Span as Span2};
use quote::quote;
use syn::{
    parse_macro_input,
    Token,
    punctuated::Punctuated,
    token::{And, Brace, Colon, Gt, Lt, Paren, PathSep, Pub},
    AngleBracketedGenericArguments, Data, DataEnum, DataStruct, DeriveInput,
    Field, FieldMutability, GenericArgument, Fields, FieldsNamed, FieldsUnnamed,
    Lifetime, Path, PathArguments, PathSegment, Type, TypePath, TypeReference,
    Variant, Visibility,
};


#[proc_macro_attribute]
pub fn contextual_error(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let DeriveInput {
        attrs: item_attrs,
        vis: item_vis,
        ident: item_ident,
        generics: item_generics,
        data: item_data
    } = parse_macro_input!(item as DeriveInput);

    let output_data = match item_data {
        Data::Union(_) => panic!("[contextual_error] Unions are not supported"),
        Data::Enum(e) => {
            todo!("Data::Enum");
            // Data::Enum(DataEnum {
            //     enum_token: e.enum_token,
            //     brace_token: e.brace_token,
            //     variants: (),
            // })
        },
        Data::Struct(s) => match s.fields {
            Fields::Named(n) => Data::Struct(DataStruct {
                struct_token: s.struct_token,
                fields: Fields::Named(FieldsNamed {
                    brace_token: Brace(Span2::call_site()),
                    named: std::iter::empty()
                        .chain(n.named.into_iter()) // user-defined fields
                        .chain(
                            vec![
                                named_location_field(),
                                named_backtrace_field(),
                            ].into_iter()
                        )
                        .collect(),
                }),
                semi_token: s.semi_token,
            }),
            Fields::Unit => Data::Struct(DataStruct {
                struct_token: s.struct_token,
                fields: Fields::Named(FieldsNamed {
                    brace_token: Brace(Span2::call_site()),
                    named: vec![
                        named_location_field(),
                        named_backtrace_field(),
                    ].into_iter().collect(),
                }),
                semi_token: s.semi_token,
            }),
            Fields::Unnamed(u) => Data::Struct(DataStruct {
                struct_token: s.struct_token,
                fields: Fields::Unnamed(FieldsUnnamed {
                    paren_token: Paren(Span2::call_site()),
                    unnamed: std::iter::empty()
                        .chain(u.unnamed.into_iter()) // user-defined fields
                        .chain(vec![
                            unnamed_location_field(),
                            unnamed_backtrace_field(),
                        ]
                               .into_iter())
                        .collect(),
                }),
                semi_token: s.semi_token,
            }),
        },
    };

    let output = DeriveInput {
        attrs: item_attrs,
        vis: item_vis,
        ident: item_ident,
        generics: item_generics,
        data: output_data
    };
    TokenStream::from(quote! {
        #output
    })
}

// pub location: &'static std::panic::Location<'static>
fn named_location_field() -> Field {
    Field {
        attrs: vec![/*none*/],
        vis: field_visibility(),
        mutability: FieldMutability::None,
        ident: Some(Ident2::new("location", Span2::call_site())),
        colon_token: Some(Colon(Span2::call_site())),
        ty: location_type(),
    }
}

// pub backtrace: std::backtrace::Backtrace
fn named_backtrace_field() -> Field {
    Field {
        attrs: vec![/*none*/],
        vis: field_visibility(),
        mutability: FieldMutability::None,
        ident: Some(Ident2::new("backtrace", Span2::call_site())),
        colon_token: Some(Colon(Span2::call_site())),
        ty: backtrace_type(),
    }
}

// pub &'static std::panic::Location<'static>
fn unnamed_location_field() -> Field {
    Field {
        attrs: vec![/*none*/],
        vis: field_visibility(),
        mutability: FieldMutability::None,
        ident: None,
        colon_token: None,
        ty: location_type(),
    }
}

// pub std::backtrace::Backtrace
fn unnamed_backtrace_field() -> Field {
    Field {
        attrs: vec![/*none*/],
        vis: field_visibility(),
        mutability: FieldMutability::None,
        ident: None,
        colon_token: None,
        ty: backtrace_type(),
    }
}

fn location_type() -> Type {
    Type::Reference(TypeReference {
        and_token: And(Span2::call_site()),
        lifetime: Some(Lifetime::new("'static", Span2::call_site())),
        mutability: None,
        elem: Box::new(Type::Path(TypePath {
            qself: None,
            path: Path {
                leading_colon: None,
                segments: vec![
                    PathSegment {
                        ident: Ident2::new("std", Span2::call_site()),
                        arguments: PathArguments::None,
                    },
                    PathSegment {
                        ident: Ident2::new("panic", Span2::call_site()),
                        arguments: PathArguments::None,
                    },
                    PathSegment {
                        ident: Ident2::new("Location", Span2::call_site()),
                        arguments: PathArguments::AngleBracketed(
                            AngleBracketedGenericArguments {
                                colon2_token: Some(PathSep::default()),
                                lt_token: Lt(Span2::call_site()),
                                args: vec![
                                    GenericArgument::Lifetime(Lifetime::new(
                                        "'static",
                                        Span2::call_site()
                                    ))
                                ].into_iter().collect(),
                                gt_token: Gt(Span2::call_site())
                            }
                        )
                    },
                ].into_iter().collect()
            },
        }))
    })
}

fn field_visibility() -> Visibility {
    Visibility::Public(Pub(Span2::call_site()))
}

fn backtrace_type() -> Type {
    Type::Path(TypePath {
        qself: None,
        path: Path {
            leading_colon: None,
            segments: vec![
                PathSegment {
                    ident: Ident2::new("std", Span2::call_site()),
                    arguments: PathArguments::None,
                },
                PathSegment {
                    ident: Ident2::new("backtrace", Span2::call_site()),
                    arguments: PathArguments::None,
                },
                PathSegment {
                    ident: Ident2::new("Backtrace", Span2::call_site()),
                    arguments: PathArguments::None
                },
            ].into_iter().collect()
        },
    })
}
