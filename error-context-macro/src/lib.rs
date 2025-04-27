//!

use proc_macro::TokenStream;
use proc_macro2::{Ident as Ident2, Span as Span2};
use quote::quote;
use syn::{
    parse_macro_input,
    token::{And, Brace, Colon, Gt, Lt, Paren, PathSep, Pub},
    AngleBracketedGenericArguments, Data, DataEnum, DataStruct, DeriveInput,
    Field, FieldMutability, GenericArgument, Fields, FieldsNamed, FieldsUnnamed,
    Lifetime, Path, PathArguments, PathSegment, Type, TypePath, TypeReference,
    Variant, Visibility,
};



// TODO: Generate a ctor for the type on which `#[contextual_error]` is used,
//       because otherwise the Location and Backtrace need to be provided
//       manually, which is super annoying.
//       For structs this can be a single ctor called `new`, while for enums it
//       should be a single ctor per enum variant carrying the variant's name.

// TODO add a feature flag to the attriubte to control enabling
//      the contextual_error attribute using a build flag defined
//      in Cargo.toml e.g. #[contextual_error(feature = "my-build-flag")]

const TAG: &str = "[contextual_error]";

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
        Data::Union(_) => panic!("{TAG} Unions are not supported"),
        Data::Enum(e) => Data::Enum(augment_enum(e)),
        Data::Struct(s) => Data::Struct(augment_struct(s)),
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

fn augment_enum(e: DataEnum) -> DataEnum {
    let output_variants = e.variants.into_iter()
        .map(|Variant { attrs, ident, fields, discriminant }| {
            if discriminant.is_some() {
                panic!("{TAG} Enum variant discriminants are not supported");
            }
            let location_name = Some(Ident2::new("location", Span2::call_site()));
            let backtrace_name = Some(Ident2::new("backtrace", Span2::call_site()));
            let output_fields = match fields {
                Fields::Named(n) => Fields::Named(FieldsNamed {
                    brace_token: n.brace_token,
                    named: std::iter::empty()
                        .chain(n.named.into_iter())
                        .chain([
                            location_field(location_name, enum_variant_field_vis()),
                            backtrace_field(backtrace_name, enum_variant_field_vis()),
                        ])
                        .collect(),
                }),
                Fields::Unit => Fields::Named(FieldsNamed {
                    brace_token: Brace(Span2::call_site()),
                    named: [
                        location_field(location_name, enum_variant_field_vis()),
                        backtrace_field(backtrace_name, enum_variant_field_vis()),
                    ].into_iter().collect(),
                }),
                Fields::Unnamed(u) => Fields::Unnamed(FieldsUnnamed {
                    paren_token: u.paren_token,
                    unnamed: std::iter::empty()
                        .chain(u.unnamed.into_iter()) // user-defined fields
                        .chain([
                            location_field(None, enum_variant_field_vis()),
                            backtrace_field(None, enum_variant_field_vis()),
                        ])
                        .collect(),
                })
            };
            Variant {
                attrs,
                ident,
                fields: output_fields,
                discriminant,
            }
        })
        .collect();
    DataEnum {
        enum_token: e.enum_token,
        brace_token: e.brace_token,
        variants: output_variants,
    }
}

fn augment_struct(s: DataStruct) -> DataStruct {
    let location_name = Some(Ident2::new("location", Span2::call_site()));
    let backtrace_name = Some(Ident2::new("backtrace", Span2::call_site()));
    match s.fields {
        Fields::Named(n) => DataStruct {
            struct_token: s.struct_token,
            fields: Fields::Named(FieldsNamed {
                brace_token: Brace(Span2::call_site()),
                named: std::iter::empty()
                    .chain(n.named.into_iter()) // user-defined fields
                    .chain([
                        location_field(location_name, struct_field_vis()),
                        backtrace_field(backtrace_name, struct_field_vis()),
                    ])
                    .collect(),
            }),
            semi_token: s.semi_token,
        },
        Fields::Unit => DataStruct {
            struct_token: s.struct_token,
            fields: Fields::Named(FieldsNamed {
                brace_token: Brace(Span2::call_site()),
                named: [
                    location_field(location_name, struct_field_vis()),
                    backtrace_field(backtrace_name, struct_field_vis()),
                ].into_iter().collect(),
            }),
            semi_token: s.semi_token,
        },
        Fields::Unnamed(u) => DataStruct {
            struct_token: s.struct_token,
            fields: Fields::Unnamed(FieldsUnnamed {
                paren_token: Paren(Span2::call_site()),
                unnamed: std::iter::empty()
                    .chain(u.unnamed.into_iter()) // user-defined fields
                    .chain([
                        location_field(None, struct_field_vis()),
                        backtrace_field(None, struct_field_vis()),
                    ])
                    .collect(),
            }),
            semi_token: s.semi_token,
        },
    }
}



// pub location: &'static std::panic::Location<'static>
fn location_field(
    field_name: Option<Ident2>,
    vis: Visibility,
) -> Field {
    Field {
        attrs: vec![/*none*/],
        vis,
        mutability: FieldMutability::None,
        ident: field_name,
        colon_token: Some(Colon(Span2::call_site())),
        ty: location_type(),
    }
}

// pub backtrace: std::backtrace::Backtrace
fn backtrace_field(
    field_name: Option<Ident2>,
    vis: Visibility,
) -> Field {
    Field {
        attrs: vec![/*none*/],
        vis,
        mutability: FieldMutability::None,
        ident: field_name,
        colon_token: Some(Colon(Span2::call_site())),
        ty: backtrace_type(),
    }
}

fn struct_field_vis() -> Visibility {
    Visibility::Public(Pub(Span2::call_site()))
}

fn enum_variant_field_vis() -> Visibility {
    Visibility::Inherited
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
                segments: [
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
                                args: [
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
