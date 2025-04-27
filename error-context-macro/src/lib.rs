//!

use proc_macro::TokenStream;
use proc_macro2::{Ident as Ident2, Span as Span2, TokenStream as TokenStream2};
use quote::quote;
use syn::{
    parse_macro_input,
    token::{And, Brace, Colon, Gt, Lt, Paren, PathSep, Pub},
    AngleBracketedGenericArguments, Data, DataEnum, DataStruct, DeriveInput,
    Field, FieldMutability, GenericArgument, Fields, FieldsNamed, FieldsUnnamed,
    Lifetime, Path, PathArguments, PathSegment, Type, TypePath, TypeReference,
    Variant, Visibility,
};


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

    let ctor_impl_block = generate_ctor_impl_block(&item_ident, &item_data);
    let output = DeriveInput {
        attrs: item_attrs,
        vis: item_vis,
        ident: item_ident,
        generics: item_generics,
        data: match &item_data {
            Data::Union(_) => panic!("{TAG} Unions are not supported"),
            Data::Enum(e) => Data::Enum(augment_enum(&e)),
            Data::Struct(s) => Data::Struct(augment_struct(&s)),
        }
    };
    TokenStream::from(quote! {
        #output
        #ctor_impl_block
    })
}

fn augment_enum(e: &DataEnum) -> DataEnum {
    let output_variants = e.variants.iter()
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
                        .chain(n.named.iter().cloned())
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
                        .chain(u.unnamed.iter().cloned()) // user-defined fields
                        .chain([
                            location_field(None, enum_variant_field_vis()),
                            backtrace_field(None, enum_variant_field_vis()),
                        ])
                        .collect(),
                })
            };
            Variant {
                attrs: attrs.clone(),
                ident: ident.clone(),
                fields: output_fields,
                discriminant: discriminant.clone(),
            }
        })
        .collect();
    DataEnum {
        enum_token: e.enum_token,
        brace_token: e.brace_token,
        variants: output_variants,
    }
}

fn augment_struct(s: &DataStruct) -> DataStruct {
    let location_name = Some(Ident2::new("location", Span2::call_site()));
    let backtrace_name = Some(Ident2::new("backtrace", Span2::call_site()));
    match &s.fields {
        Fields::Named(n) => DataStruct {
            struct_token: s.struct_token,
            fields: Fields::Named(FieldsNamed {
                brace_token: Brace(Span2::call_site()),
                named: std::iter::empty()
                    .chain(n.named.iter().cloned()) // user-defined fields
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
                    .chain(u.unnamed.iter().cloned()) // user-defined fields
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

fn generate_ctor_impl_block(type_name: &Ident2, item_data: &Data) -> TokenStream2 {
    match &item_data {
        Data::Union(_) => panic!("{TAG} Unions are not supported"),
        Data::Enum(e) => {
            let enum_ctors = generate_enum_ctors(&e);
            quote! {
                impl #type_name {
                    #( #enum_ctors )*
                }
            }
        },
        Data::Struct(s) => {
            let struct_ctor = generate_struct_ctor(&s);
            quote! {
                impl #type_name {
                    #struct_ctor
                }
            }
        },
    }
}

fn generate_struct_ctor(s: &DataStruct) -> TokenStream2 {
    match &s.fields {
        Fields::Named(n) => {
            let params = n.named.iter()
                .map(|Field { ident, ty, .. }| quote! { #ident : #ty });
            let field_initializers = n.named.iter()
                .map(|Field { ident, ty: _, .. }| quote! { #ident })
                .chain([
                    quote! { location: std::panic::Location::caller()        },
                    quote! { backtrace: std::backtrace::Backtrace::capture() },
                ]);
            quote! {
                fn new( #(#params),* ) -> Self {
                    Self {
                        #(#field_initializers),*
                    }
                }
            }
        }
        Fields::Unit => {
            let field_initializers = std::iter::empty()
                .chain([
                    quote! { location: std::panic::Location::caller()        },
                    quote! { backtrace: std::backtrace::Backtrace::capture() },
                ]);
            quote! {
                fn new() -> Self {
                    Self {
                        #(#field_initializers),*
                    }
                }
            }
        },
        Fields::Unnamed(u) => {
            let params = u.unnamed.iter().enumerate()
                .map(|(i, Field { ident: _, ty, .. })| {
                    let ident = format!("field{i}");
                    let ident = Ident2::new(&ident, Span2::call_site());
                    quote! { #ident : #ty }
                });
            let field_initializers = u.unnamed.iter().enumerate()
                .map(|(i, Field { ident: _, ty: _, .. })| {
                    let ident = format!("field{i}");
                    let ident = Ident2::new(&ident, Span2::call_site());
                    quote! { #ident }
                })
                .chain([
                    quote! { std::panic::Location::caller()       },
                    quote! { std::backtrace::Backtrace::capture() },
                ]);
            quote! {
                fn new( #(#params),* ) -> Self {
                    Self(
                        #(#field_initializers),*
                    )
                }
            }
        }
    }
}

fn generate_enum_ctors(e: &DataEnum) -> Vec<TokenStream2> {
    e.variants.iter()
        .map(|Variant { ident, fields, .. }| {
            let variant_name = ident;
            let ctor_name = format!("new_{ident}");
            let ctor_name = Ident2::new(&ctor_name, Span2::call_site());
            match fields {
                Fields::Named(n) => {
                    let params = n.named.iter()
                        .map(|Field { ident, ty, .. }| quote! { #ident : #ty });
                    let field_initializers = n.named.iter()
                        .map(|Field { ident, ty: _, .. }| quote! { #ident })
                        .chain([
                            quote! { location: std::panic::Location::caller()        },
                            quote! { backtrace: std::backtrace::Backtrace::capture() },
                        ]);
                    quote! {
                        pub fn #ctor_name( #(#params),* ) -> Self {
                            Self::#variant_name {
                                #(#field_initializers),*
                            }
                        }
                    }
                },
                Fields::Unit => {
                    let field_initializers = std::iter::empty()
                        .chain([
                            quote! { location: std::panic::Location::caller()        },
                            quote! { backtrace: std::backtrace::Backtrace::capture() },
                        ]);
                    quote! {
                        pub fn #ctor_name() -> Self {
                            Self::#variant_name {
                                #(#field_initializers),*
                            }
                        }
                    }
                },
                Fields::Unnamed(u) => {
                    let params = u.unnamed.iter().enumerate()
                        .map(|(i, Field { ident: _, ty, .. })| {
                            let ident = format!("field{i}");
                            let ident = Ident2::new(&ident, Span2::call_site());
                            quote! { #ident : #ty }
                        });
                    let field_initializers = u.unnamed.iter().enumerate()
                        .map(|(i, Field { ident: _, ty: _, .. })| {
                            let ident = format!("field{i}");
                            let ident = Ident2::new(&ident, Span2::call_site());
                            quote! { #ident }
                        })
                        .chain([
                            quote! { std::panic::Location::caller()       },
                            quote! { std::backtrace::Backtrace::capture() },
                        ]);
                    quote! {
                        pub fn #ctor_name( #(#params),* ) -> Self {
                            Self::#variant_name(
                                #(#field_initializers),*
                            )
                        }
                    }
                },
            }
        })
        .collect()
}
