//!

use proc_macro::{token_stream::IntoIter, TokenStream, TokenTree};
use proc_macro2::{
    Ident as Ident2, Punct as Punct2, Spacing as Spacing2, Span as Span2,
    TokenStream as TokenStream2, TokenTree as TokenTree2,
};
use quote::{quote, ToTokens};
use std::iter::Peekable;
use syn::{
    parse_macro_input,
    token::{Brace, Bracket, Colon, Paren, Pound, Pub},
    Attribute, AttrStyle, Data, DataEnum, DataStruct, DeriveInput, Expr, Field,
    FieldMutability, Fields, FieldsNamed, FieldsUnnamed, MacroDelimiter, Meta,
    MetaList, Path, PathArguments, PathSegment, Type, TypePath, Variant,
    Visibility,
};

#[proc_macro_attribute]
pub fn contextual_error(attr: TokenStream, item: TokenStream) -> TokenStream {
    let DeriveInput {
        attrs: item_attrs,
        vis: item_vis,
        ident: item_ident,
        generics: item_generics,
        data: item_data
    } = parse_macro_input!(item as DeriveInput);
    let field_attrs = FieldAttrs::parse(attr).to_attr_vec();
    let ctor_impl_block = generate_ctor_impl_block(
        &item_ident,
        &item_data,
        &field_attrs,
    );
    let augmented_type = DeriveInput {
        attrs: item_attrs,
        vis: item_vis,
        ident: item_ident,
        generics: item_generics,
        data: match &item_data {
            Data::Union(_) => panic!("Unions are not supported"),
            Data::Enum(e) => Data::Enum(augment_enum(&e, &field_attrs)),
            Data::Struct(s) => Data::Struct(augment_struct(&s, &field_attrs)),
        }
    };
    TokenStream::from(quote! {
        #augmented_type
        #ctor_impl_block
    })
}

fn augment_enum(e: &DataEnum, field_attrs: &[Attribute]) -> DataEnum {
    let output_variants = e.variants.iter()
        .map(|Variant { attrs, ident, fields, discriminant }| {
            if discriminant.is_some() {
                panic!("Enum variant discriminants are not supported");
            }
            let field_name = Some(Ident2::new("ctx", Span2::call_site()));
            let field_vis = Visibility::Inherited;
            let output_fields = match fields {
                Fields::Named(n) => Fields::Named(FieldsNamed {
                    brace_token: n.brace_token,
                    named: std::iter::empty()
                        .chain(n.named.iter().cloned())
                        .chain([ ctx_field(field_attrs, field_vis, field_name) ])
                        .collect(),
                }),
                Fields::Unit => Fields::Named(FieldsNamed {
                    brace_token: Brace(Span2::call_site()),
                    named: std::iter::empty()
                        .chain([ ctx_field(field_attrs, field_vis, field_name) ])
                        .collect(),
                }),
                Fields::Unnamed(u) => Fields::Unnamed(FieldsUnnamed {
                    paren_token: u.paren_token,
                    unnamed: std::iter::empty()
                        .chain(u.unnamed.iter().cloned()) // user-defined fields
                        .chain([ ctx_field(field_attrs, field_vis, None) ])
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

fn augment_struct(s: &DataStruct, field_attrs: &[Attribute]) -> DataStruct {
    let field_vis = Visibility::Public(Pub(Span2::call_site()));
    let field_name = Some(Ident2::new("ctx", Span2::call_site()));
    match &s.fields {
        Fields::Named(n) => DataStruct {
            struct_token: s.struct_token,
            fields: Fields::Named(FieldsNamed {
                brace_token: Brace(Span2::call_site()),
                named: std::iter::empty()
                    .chain(n.named.iter().cloned()) // user-defined fields
                    .chain([
                        ctx_field(field_attrs, field_vis, field_name),
                    ])
                    .collect(),
            }),
            semi_token: s.semi_token,
        },
        Fields::Unit => DataStruct {
            struct_token: s.struct_token,
            fields: Fields::Named(FieldsNamed {
                brace_token: Brace(Span2::call_site()),
                named: std::iter::empty()
                    .chain([ ctx_field(field_attrs, field_vis, field_name) ])
                    .collect(),
            }),
            semi_token: s.semi_token,
        },
        Fields::Unnamed(u) => DataStruct {
            struct_token: s.struct_token,
            fields: Fields::Unnamed(FieldsUnnamed {
                paren_token: Paren(Span2::call_site()),
                unnamed: std::iter::empty()
                    .chain(u.unnamed.iter().cloned()) // user-defined fields
                    .chain([ ctx_field(field_attrs, field_vis, None) ])
                    .collect(),
            }),
            semi_token: s.semi_token,
        },
    }
}

// pub ctx: error_context_core::ErrorCtx
fn ctx_field(
    field_attrs: &[Attribute],
    vis: Visibility,
    field_name: Option<Ident2>,
) -> Field {
    Field {
        attrs: field_attrs.to_vec(),
        vis,
        mutability: FieldMutability::None,
        ident: field_name,
        colon_token: Some(Colon(Span2::call_site())),
        ty: Type::Path(TypePath {
            qself: None,
            path: Path {
                leading_colon: None,
                segments: vec![
                    PathSegment {
                        ident: Ident2::new("error_context", Span2::call_site()),
                        arguments: PathArguments::None,
                    },
                    PathSegment {
                        ident: Ident2::new("ErrorCtx", Span2::call_site()),
                        arguments: PathArguments::None
                    },
                ].into_iter().collect()
            },
        })
    }
}

fn generate_ctor_impl_block(
    type_name: &Ident2,
    item_data: &Data,
    field_attrs: &[Attribute],
) -> TokenStream2 {
    match &item_data {
        Data::Union(_) => panic!("Unions are not supported"),
        Data::Enum(e) => {
            let enum_ctors = generate_enum_ctors(&e, field_attrs);
            quote! {
                impl #type_name {
                    #( #enum_ctors )*
                }
            }
        },
        Data::Struct(s) => {
            let struct_ctor = generate_struct_ctor(&s, field_attrs);
            quote! {
                impl #type_name {
                    #struct_ctor
                }
            }
        },
    }
}

fn generate_struct_ctor(
    s: &DataStruct,
    field_attrs: &[Attribute],
) -> TokenStream2 {
    match &s.fields {
        Fields::Named(n) => {
            let params = n.named.iter()
                .map(|Field { ident, ty, .. }| quote! { #ident : #ty });
            let field_initializers = n.named.iter()
                .map(|Field { ident, ty: _, .. }| quote! { #ident })
                .chain([
                    quote! {
                        #(#field_attrs)*
                        ctx: error_context::ErrorCtx::new(),
                    },
                ]);
            quote! {
                #[track_caller]
                pub fn new( #(#params),* ) -> Self {
                    Self {
                        #(#field_initializers),*
                    }
                }
            }
        },
        Fields::Unit => {
            let field_initializers = std::iter::empty()
                .chain([
                    quote! {
                        #(#field_attrs)*
                        ctx: error_context::ErrorCtx::new(),
                    },
                ]);
            quote! {
                #[track_caller]
                pub fn new() -> Self {
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
                    quote! {
                        #(#field_attrs)*
                        error_context::ErrorCtx::new(),
                    },
                ]);
            quote! {
                #[track_caller]
                pub fn new( #(#params),* ) -> Self {
                    Self(
                        #(#field_initializers),*
                    )
                }
            }
        }
    }
}

fn generate_enum_ctors(
    e: &DataEnum,
    field_attrs: &[Attribute],
) -> Vec<TokenStream2> {
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
                            quote! {
                                #(#field_attrs)*
                                ctx: error_context::ErrorCtx::new(),
                            },
                        ]);
                    quote! {
                        #[track_caller]
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
                            quote! {
                                #(#field_attrs)*
                                ctx: error_context::ErrorCtx::new(),
                            },
                        ]);
                    quote! {
                        #[track_caller]
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
                            quote! {
                                #(#field_attrs)*
                                error_context::ErrorCtx::new()
                            },
                        ]);
                    quote! {
                        #[track_caller]
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


struct FieldAttrs {
    build_flag: Option<FeatureFlagAttr>,
}

impl FieldAttrs {
    fn parse(attr: TokenStream) -> Self {
        let mut attr_iter = attr.into_iter().peekable();
        let mut field_attrs = Self {
            build_flag: None
        };
        while let Some(tt) = attr_iter.peek() {
            match &*tt.to_string() {
                "feature" => {
                    let arg = FeatureFlagAttr::parse(&mut attr_iter);
                    field_attrs.build_flag = Some(arg);
                },
                _ => panic!("Expected attr name 'feature'")
            }
            // TODO comma for if/when there's more than 1 attr argument
        }
        field_attrs
    }

    fn to_attr_vec(&self) -> Vec<Attribute> {
        let mut vec = vec![];
        if let Some(build_flag) = &self.build_flag {
            vec.push(Attribute {
                pound_token: Pound(Span2::call_site()),
                style: AttrStyle::Outer,
                bracket_token: Bracket(Span2::call_site()),
                meta: Meta::List(MetaList {
                    path: Path {
                        leading_colon: None,
                        segments: [
                            PathSegment {
                                ident: Ident2::new("cfg", Span2::call_site()),
                                arguments: PathArguments::None,
                            },
                        ].into_iter().collect(),
                    },
                    delimiter: MacroDelimiter::Paren(
                        Paren(Span2::call_site())
                    ),
                    tokens: {
                        let mut token_stream = build_flag.name_stream.clone();
                        token_stream.extend(TokenStream2::from(
                            TokenTree2::Punct(Punct2::new('=', Spacing2::Alone))
                        ));
                        token_stream.extend(build_flag.value.to_token_stream());
                        token_stream
                    },
                }),
            });
        }
        vec
    }
}


// Currently ONLY recognizes the attribute arguments:
// - feature = "<BUILD_FEATURE_NAME>"
struct FeatureFlagAttr {
    #[allow(unused)]
    name: Ident2,
    name_stream: TokenStream2,
    value: Expr,
}

impl FeatureFlagAttr {
    fn parse(attr_iter: &mut Peekable<IntoIter>) -> Self {
        let attr_name_tt: TokenTree = attr_iter.next().unwrap();
        assert_eq!(attr_name_tt.to_string(), "feature");

        let name_stream: TokenStream2 = TokenStream::from(attr_name_tt).into();
        let name: Ident2 = syn::parse2(name_stream.clone()).unwrap_or_else(|_| {
            panic!("Failed to parse feature attribute")
        });

        let _eq_tt = attr_iter.next();
        let _eq = match _eq_tt {
            Some(TokenTree::Punct(p)) => assert_eq!(p.as_char(), '='),
            Some(tt) => panic!("Unrecognized token tree: {tt}"),
            None => panic!("Expected '='"),
        };

        let value_tt = attr_iter.next();
        let value: Expr = match value_tt {
            Some(tt @ TokenTree::Literal(_)) => {
                let stream: TokenStream2 = TokenStream::from(tt).into();
                let expr: Expr = syn::parse2(stream).unwrap_or_else(|_| {
                    panic!("Failed to parse attr key-value pair")
                });
                expr
            }
            Some(tt) => panic!("Expected attr value, got token tree {tt}"),
            None => panic!("Expected attr value"),
        };

        Self { name, name_stream, value }
    }
}
