//!
#![allow(non_snake_case)]

use proc_macro::{Delimiter, TokenStream, TokenTree, token_stream::IntoIter};
use proc_macro2::{
    Ident as Ident2, Punct as Punct2, Spacing as Spacing2, Span as Span2,
    TokenStream as TokenStream2, TokenTree as TokenTree2,
};
use quote::quote;
use regex::{Match, Regex};
use std::collections::HashMap;
use std::iter::Peekable;
use std::ops::Range;
use std::sync::LazyLock;
use syn::{
    AttrStyle, Attribute, Data, DataEnum, DataStruct, DeriveInput, Expr,
    ExprLit, Field, FieldMutability, Fields, FieldsNamed, FieldsUnnamed, Lit,
    LitInt, LitStr, MacroDelimiter, Meta, MetaList, Path, PathArguments,
    PathSegment, Type, TypePath, Variant, Visibility, parse_macro_input,
    token::{Brace, Bracket, Colon, Paren, Pound, Pub},
};

#[proc_macro_attribute]
pub fn err_marks_the_spot(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item2 = item.clone();
    let type_item @ DeriveInput {
        attrs: item_attrs,
        vis: item_vis,
        ident: item_ident,
        generics: item_generics,
        data: item_data,
    } = &parse_macro_input!(item2 as DeriveInput);

    let type_attr_args = TypeAttrArgs::parse(attr);
    let field_attrs = type_attr_args.field_attr_vec();
    let ctor_attrs = type_attr_args.ctor_attr_vec();
    let impl_ctors_for_type = generate_ctor_impl_block(
        &ctor_attrs,
        &item_ident,
        &item_data,
        &field_attrs,
    );

    let augmented_type_item = DeriveInput {
        attrs: item_attrs.clone(),
        vis: item_vis.clone(),
        ident: item_ident.clone(),
        generics: item_generics.clone(),
        data: match &item_data {
            Data::Union(_) => panic!("Unions are not supported"),
            Data::Enum(e) => Data::Enum(augment_enum(
                type_attr_args.build_feature.as_ref(),
                &e,
                &field_attrs,
            )),
            Data::Struct(s) => Data::Struct(augment_struct(
                type_attr_args.build_feature.as_ref(),
                &s,
                &field_attrs,
            )),
        },
    };

    let impl_Display_for_type: TokenStream2 = gen_impl_Display_for_type(
        type_attr_args.build_feature.as_ref(),
        &type_item,
    );

    TokenStream::from(quote! {
        #augmented_type_item
        #impl_ctors_for_type
        #impl_Display_for_type
    })
}

#[rustfmt::skip]
fn augment_enum(
    build_feature: Option<&BuildFeatureAttr>,
    e: &DataEnum,
    field_attrs: &[Attribute],
) -> DataEnum {
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
                        .chain(vec![
                            ctx_field(
                                build_feature,
                                field_attrs,
                                field_vis,
                                field_name
                            )
                        ])
                        .collect(),
                }),
                Fields::Unit => Fields::Named(FieldsNamed {
                    brace_token: Brace(Span2::call_site()),
                    named: std::iter::empty()
                        .chain(vec![
                            ctx_field(
                                build_feature,
                                field_attrs,
                                field_vis,
                                field_name
                            )
                        ])
                        .collect(),
                }),
                Fields::Unnamed(u) => Fields::Unnamed(FieldsUnnamed {
                    paren_token: u.paren_token,
                    unnamed: std::iter::empty()
                        .chain(u.unnamed.iter().cloned()) // user-defined fields
                        .chain(vec![
                            ctx_field(
                                build_feature,
                                field_attrs,
                                field_vis,
                                None
                            )
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

#[rustfmt::skip]
fn augment_struct(
    build_feature: Option<&BuildFeatureAttr>,
    s: &DataStruct,
    field_attrs: &[Attribute],
) -> DataStruct {
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
                        ctx_field(
                            build_feature,
                            field_attrs,
                            field_vis,
                            field_name
                        ),
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
                    .chain([
                        ctx_field(
                            build_feature,
                            field_attrs,
                            field_vis,
                            field_name
                        ),
                    ])
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
                    .chain([
                        ctx_field(
                            build_feature,
                            field_attrs,
                            field_vis,
                            None
                        ),
                    ])
                    .collect(),
            }),
            semi_token: s.semi_token,
        },
    }
}

// pub ctx: error_context_core::ErrorCtx
fn ctx_field(
    build_feature: Option<&BuildFeatureAttr>,
    field_attrs: &[Attribute],
    vis: Visibility,
    field_name: Option<Ident2>,
) -> Field {
    Field {
        attrs: field_attrs.to_vec().into_iter()
            .chain(if let Some(feature) = build_feature {
                vec![ feature.to_attr() ]
            } else {
                vec![/* Don't add #[cfg(feature = <FEATURE>)] attribure */]
            })
            .collect(),
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
                        ident: Ident2::new("err_marks_the_spot", Span2::call_site()),
                        arguments: PathArguments::None,
                    },
                    PathSegment {
                        ident: Ident2::new("ErrorCtx", Span2::call_site()),
                        arguments: PathArguments::None,
                    },
                ].into_iter().collect()
            },
        }),
    }
}

fn generate_ctor_impl_block(
    ctor_attrs: &[Attribute],
    type_name: &Ident2,
    item_data: &Data,
    field_attrs: &[Attribute],
) -> TokenStream2 {
    match &item_data {
        Data::Union(_) => panic!("Unions are not supported"),
        Data::Enum(e) => {
            let enum_ctors = generate_enum_ctors(&e, field_attrs, ctor_attrs);
            quote! {
                impl #type_name {
                    #( #enum_ctors )*
                }
            }
        },
        Data::Struct(s) => {
            let struct_ctor = generate_struct_ctor(&s, field_attrs, ctor_attrs);
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
    ctor_attrs: &[Attribute],
) -> TokenStream2 {
    match &s.fields {
        Fields::Named(n) => {
            let params = n.named.iter()
                .map(|Field { ident, ty, .. }| quote! { #ident : impl Into<#ty> });
            let field_initializers = n.named.iter()
                .map(|Field { ident, ty: _, .. }| quote! { #ident: #ident.into() })
                .chain([
                    quote! {
                        #(#field_attrs)*
                        ctx: err_marks_the_spot::ErrorCtx::new(),
                    },
                ]);
            quote! {
                #(#ctor_attrs)*
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
                        ctx: err_marks_the_spot::ErrorCtx::new(),
                    },
                ]);
            quote! {
                #(#ctor_attrs)*
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
                    quote! { #ident : impl Into<#ty> }
                });
            let field_initializers = u.unnamed.iter().enumerate()
                .map(|(i, Field { ident: _, ty: _, .. })| {
                    let ident = format!("field{i}");
                    let ident = Ident2::new(&ident, Span2::call_site());
                    quote! { #ident.into() }
                })
                .chain([
                    quote! {
                        #(#field_attrs)*
                        err_marks_the_spot::ErrorCtx::new(),
                    },
                ]);
            quote! {
                #(#ctor_attrs)*
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
    ctor_attrs: &[Attribute],
) -> Vec<TokenStream2> {
    e.variants.iter()
        .map(|Variant { ident, fields, .. }| {
            let variant_name = ident;
            let ctor_name = format!("new_{ident}");
            let ctor_name = Ident2::new(&ctor_name, Span2::call_site());
            match fields {
                Fields::Named(n) => {
                    let params = n.named.iter()
                        .map(|Field { ident, ty, .. }| {
                            quote! { #ident : impl Into<#ty> }
                        });
                    let field_initializers = n.named.iter()
                        .map(|Field { ident, ty: _, .. }| {
                            quote! { #ident: #ident.into() }
                        })
                        .chain([
                            quote! {
                                #(#field_attrs)*
                                ctx: err_marks_the_spot::ErrorCtx::new(),
                            },
                        ]);
                    quote! {
                        #(#ctor_attrs)*
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
                                ctx: err_marks_the_spot::ErrorCtx::new(),
                            },
                        ]);
                    quote! {
                        #(#ctor_attrs)*
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
                            quote! { #ident : impl Into<#ty> }
                        });
                    let field_initializers = u.unnamed.iter().enumerate()
                        .map(|(i, Field { ident: _, ty: _, .. })| {
                            let ident = format!("field{i}");
                            let ident = Ident2::new(&ident, Span2::call_site());
                            quote! { #ident.into() }
                        })
                        .chain([
                            quote! {
                                #(#field_attrs)*
                                err_marks_the_spot::ErrorCtx::new()
                            },
                        ]);
                    quote! {
                        #(#ctor_attrs)*
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

#[derive(Debug)]
struct TypeAttrArgs {
    build_feature: Option<BuildFeatureAttr>,
    inline_ctors: Option<InlineCtorsAttr>,
}

impl TypeAttrArgs {
    fn parse(attr: TokenStream) -> Self {
        let mut attr_iter = attr.into_iter().peekable();
        let mut field_attrs = Self {
            build_feature: None,
            inline_ctors: None,
        };
        let mut loop_count = 0;
        while let Some(tt) = attr_iter.peek() {
            let peeked = tt.to_string();
            // TODO
            // if loop_count > 0 {
            //     attr_arg::parse_comma_token(&mut attr_iter);
            // }
            match &*peeked {
                "," if loop_count > 0 => {
                    attr_arg::parse_comma_token(&mut attr_iter);
                }
                "feature" => {
                    let arg = BuildFeatureAttr::parse(&mut attr_iter);
                    field_attrs.build_feature = Some(arg);
                },
                "inline_ctors" => {
                    let arg = InlineCtorsAttr::parse(&mut attr_iter);
                    field_attrs.inline_ctors = Some(arg);
                },
                _ => panic!("Expected attr name 'feature', 'inline_ctors', got {peeked}")
            }
            loop_count += 1;
        }
        field_attrs
    }

    fn field_attr_vec(&self) -> Vec<Attribute> {
        let mut vec = vec![];
        if let Some(build_flag) = &self.build_feature {
            vec.push(build_flag.to_attr());
        }
        vec
    }

    fn ctor_attr_vec(&self) -> Vec<Attribute> {
        let mut vec = vec![];
        if let Some(inline_ctors) = &self.inline_ctors {
            vec.push(inline_ctors.to_attr());
        }
        vec
    }
}

// Currently ONLY recognizes the attribute arguments:
// - feature = "<BUILD_FEATURE_NAME>"
#[allow(unused)]
#[derive(Debug)]
struct BuildFeatureAttr {
    name: Ident2,
    name_stream: TokenStream2,
    value: Expr,
}

impl BuildFeatureAttr {
    fn parse(attr_iter: &mut Peekable<IntoIter>) -> Self {
        let attr_arg_name = "feature";
        let (name, name_stream) = attr_arg::parse_name(attr_iter, attr_arg_name);
        attr_arg::parse_eq_token(attr_iter, &attr_arg_name);
        let value: Expr = attr_arg::parse_value_expr(attr_iter, attr_arg_name);
        Self { name, name_stream, value }
    }

    fn to_attr(&self) -> Attribute {
        Attribute {
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
                delimiter: MacroDelimiter::Paren(Paren(Span2::call_site())),
                tokens: {
                    let mut stream = TokenStream2::new();
                    stream.extend({
                        let feature = Ident2::new("feature", Span2::call_site());
                        let feature = TokenTree2::Ident(feature);
                        TokenStream2::from(feature.clone())
                    });
                    stream.extend({
                        let eq = Punct2::new('=', Spacing2::Alone);
                        let eq = TokenTree2::Punct(eq);
                        TokenStream2::from(eq)
                    });
                    let feature = &self.value;
                    stream.extend(quote! {
                        #feature
                    });
                    stream
                },
            }),
        }
    }
}

// Currently ONLY recognizes the attribute arguments:
// - inline_ctors = true | false
#[derive(Debug)]
struct InlineCtorsAttr {
    #[allow(unused)]
    name: Ident2,
    value: Option<Ident2>,
}

impl InlineCtorsAttr {
    fn parse(attr_iter: &mut Peekable<IntoIter>) -> Self {
        let attr_arg_name = "inline_ctors";
        let (name, _stream) = attr_arg::parse_name(attr_iter, attr_arg_name);
        let value = attr_arg::parse_parenthesized_value_ident(attr_iter);
        Self { name, value }
    }

    fn to_attr(&self) -> Attribute {
        Attribute {
            pound_token: Pound(Span2::call_site()),
            style: AttrStyle::Outer,
            bracket_token: Bracket(Span2::call_site()),
            meta: if let Some(value) = self.value.as_ref() {
                Meta::List(MetaList {
                    path: Path {
                        leading_colon: None,
                        segments: [
                            PathSegment {
                                ident: Ident2::new("inline", Span2::call_site()),
                                arguments: PathArguments::None,
                            },
                        ].into_iter().collect(),
                    },
                    delimiter: MacroDelimiter::Paren(Paren(Span2::call_site())),
                    tokens: TokenStream2::from(TokenTree2::Ident(value.clone())),
                })
            } else {
                Meta::Path(Path {
                    leading_colon: None,
                    segments: [
                        PathSegment {
                            ident: Ident2::new("inline", Span2::call_site()),
                            arguments: PathArguments::None,
                        },
                    ].into_iter().collect(),
                })
            },
        }
    }
}

mod attr_arg {
    use super::*;

    pub fn parse_name(
        attr_iter: &mut Peekable<IntoIter>,
        attr_arg_name: &str,
    ) -> (Ident2, TokenStream2) {
        let attr_name_tt: TokenTree = attr_iter.next().unwrap();
        assert_eq!(attr_name_tt.to_string(), attr_arg_name);
        let name_stream: TokenStream2 = TokenStream::from(attr_name_tt).into();
        let name: Ident2 = syn::parse2(name_stream.clone()).unwrap_or_else(|_| {
            panic!("Failed to parse attribute argument: {attr_arg_name}")
        });
        (name, name_stream)
    }

    pub fn parse_eq_token(
        attr_iter: &mut Peekable<IntoIter>,
        attr_arg_name: &str,
    ) {
        let _eq_tt = attr_iter.next();
        let _eq = match _eq_tt {
            Some(TokenTree::Punct(p)) => {
                assert_eq!(p.as_char(), '=', "{attr_arg_name}");
            },
            Some(tt) => panic!("Unrecognized token tree: {tt}"),
            None => panic!("Expected '='"),
        };
    }

    pub fn parse_value_expr(
        attr_iter: &mut Peekable<IntoIter>,
        attr_arg_name: &str,
    ) -> Expr {
        match attr_iter.next() {
            Some(tt @ TokenTree::Literal(_)) => {
                let stream: TokenStream2 = TokenStream::from(tt).into();
                syn::parse2::<Expr>(stream).unwrap_or_else(|_| panic!(
                    "Failed to parse value expr of attribute argument {}",
                    attr_arg_name
                ))
            }
            Some(tt) => panic!(
                "Expected value expr of attribute argument {}, got token tree {}",
                attr_arg_name, tt
            ),
            None => panic!(
                "Expected value expr of attribute argument {}",
                attr_arg_name,
            ),
        }
    }

    pub fn parse_parenthesized_value_ident(
        attr_iter: &mut Peekable<IntoIter>,
    ) -> Option<Ident2> {
        match attr_iter.next() {
            Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Parenthesis => {
                let inner_stream = TokenStream2::from(g.stream());
                let attr_arg_value = syn::parse2::<Ident2>(inner_stream)
                    .expect("Expected a parenthesized value ident");
                Some(attr_arg_value)
            },
            _ => None,
        }
    }

    pub fn parse_comma_token(attr_iter: &mut Peekable<IntoIter>) {
        let _comma_tt = attr_iter.next();
        let _comma = match _comma_tt {
            Some(TokenTree::Punct(p)) => assert_eq!(p.as_char(), ','),
            Some(tt) => panic!("Unrecognized token tree: {tt}"),
            None => panic!("Expected ','"),
        };
    }
}

fn gen_impl_Display_for_type(
    build_feature: Option<&BuildFeatureAttr>,
    type_item: &DeriveInput,
) -> TokenStream2 {
    let type_item_name = &type_item.ident;
    let type_item_docstrs: Vec<String> = get_docstrs_from_attrs(&type_item.attrs);
    let item_field_map: FieldMap = match &type_item.data {
        Data::Union(_) => panic!("Unions are not supported"),
        Data::Enum(e) => {
            let enum_fields_map = e.variants
                .iter()
                .map(|Variant { ident, fields, .. }| (
                    ident.clone(),
                    create_fields_map(&fields)
                ))
                .collect();
            FieldMap::Enum(enum_fields_map)
        }
        Data::Struct(s) => FieldMap::Struct(create_fields_map(&s.fields)),
    };
    let struct_impl_Display_contents = get_struct_impl_Display_contents(
        build_feature,
        type_item,
        &type_item_docstrs,
        &item_field_map,
    );
    let enum_impl_Display_contents = get_enum_impl_Display_contents(
        build_feature,
        type_item,
        &item_field_map,
    );
    let impl_Display_contents = if let FieldMap::Struct(_) = item_field_map {
        quote! { #struct_impl_Display_contents }
    } else {
        quote! { #enum_impl_Display_contents }
    };
    quote! {
        impl std::fmt::Display for #type_item_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                #impl_Display_contents
                Ok(())
            }
        }
    }
}

fn get_struct_impl_Display_contents(
    build_feature: Option<&BuildFeatureAttr>,
    type_item: &DeriveInput,
    type_item_docstrs: &[String],
    item_field_map: &FieldMap,
) -> TokenStream2 {
    let type_item_name = &type_item.ident;
    let quotes: Vec<TokenStream2> = type_item_docstrs.iter()
        .filter(|_| matches!(item_field_map, FieldMap::Struct(_)))
        .map(|item_docstr| {
            let item_docstr_fields = find_docstring_fields(item_docstr);
            let modified_item_docstr = modify_docstr(
                &item_docstr,
                &item_docstr_fields
            );
            let trimmed_item_docstr = LitStr::new(
                &modified_item_docstr,
                Span2::call_site()
            );
            assert!(matches!(type_item.data, Data::Struct(_)));
            let FieldMap::Struct(field_map) = &item_field_map else { unreachable!() };
            let fields: Vec<&FieldIdToken> = item_docstr_fields.iter()
                .map(|(_pos, field_name)| {
                    field_map.get(*field_name).unwrap_or_else(|| panic!(
                        "Type {} no has no field '{}'",
                        type_item_name, field_name
                    ))
                })
                .collect();
            quote! {
                writeln!(
                    f,
                    #trimmed_item_docstr,
                    #(&self . #fields),*
                )?;
            }
        })
        .chain([
            if let Some(BuildFeatureAttr { name, value, .. }) = build_feature {
                assert_eq!(name, &Ident2::new("feature", Span2::call_site()));
                // Write an empty line between original msg & ErrorCtx, but
                // only perform the writeln!() call if the consumer crate is
                // built with the build feature enabled:
                quote! {
                    #[cfg(feature = #value)]
                    writeln!(f, "")?;
                }
            } else {
                // Write an empty line between original msg & ErrorCtx:
                quote! { writeln!(f, "")?; }
            },
        ])
        .chain(if let Data::Struct(s) = &type_item.data {
            // ErrorCtx docstring extension:
            vec![writeln_for_ErrorCtx_field(build_feature, &s.fields)]
        } else {
            vec![]
        })
        .collect();
    quote! { #(#quotes)* }
}

fn get_enum_impl_Display_contents(
    build_feature: Option<&BuildFeatureAttr>,
    type_item: &DeriveInput,
    item_field_map: &FieldMap,
) -> TokenStream2 {
    let FieldMap::Enum(field_map) = &item_field_map else { return quote!{} };
    let Data::Enum(data) = &type_item.data else { return quote!{} };
    let DataEnum { variants, .. } = data;

    let variant_writelns: Vec<_> = variants.iter()
        .map(|Variant { attrs, ident: variant_name, fields, .. }| {
            let vdocstrs = get_docstrs_from_attrs(attrs);

            let vbindings: Vec<TokenStream2> = match fields {
                Fields::Named(n) => n.named.iter()
                    .map(|field| field.ident.clone().unwrap())
                    .map(|field_name| quote! { #field_name })
                    .collect(),
                Fields::Unit => vec![],
                Fields::Unnamed(u) => u.unnamed.iter()
                    .enumerate()
                    .map(|(i, field)| {
                        field.ident.clone().unwrap_or_else(|| {
                            Ident2::new(
                                &format!("f{i}"),
                                Span2::call_site()
                            )
                        })
                    })
                    .map(|field_name| quote! { #field_name })
                    .collect(),
            };

            let vbind_list = match fields {
                Fields::Named(_)   => quote! { { #(#vbindings ,)*      } },
                Fields::Unit       => quote! { { #(#vbindings ,)*      } },
                Fields::Unnamed(_) => quote! { ( #(#vbindings ,)*      ) },
            };
            let vbind_list_with_ctx = match fields {
                Fields::Named(_)   => quote! { { #(#vbindings ,)* ctx, } },
                Fields::Unit       => quote! { { #(#vbindings ,)* ctx, } },
                Fields::Unnamed(_) => quote! { ( #(#vbindings ,)* ctx, ) },
            };

            let vdocstr_writelns: Vec<TokenStream2> = vdocstrs.iter()
                .map(|variant_docstr| {
                    let variant_docstr_fields = find_docstring_fields(
                        variant_docstr
                    );
                    let modified_variant_docstr = modify_docstr(
                        &variant_docstr,
                        &variant_docstr_fields
                    );
                    let trimmed_variant_docstr = LitStr::new(
                        &modified_variant_docstr,
                        Span2::call_site()
                    );
                    let variant_docstr_fields: Vec<Ident2> =
                        variant_docstr_fields
                        .iter()
                        .map(|(_pos, field_name)| {
                            let enum_field_map = field_map.get(variant_name)
                                .unwrap_or_else(|| panic!(
                                    "Type {} no has no variant '{}'",
                                    type_item.ident, variant_name
                                ));
                            let field_token = enum_field_map
                                .get(*field_name)
                                .unwrap_or_else(|| panic!(
                                    "Type variant {}::{} no has no field '{}'",
                                    type_item.ident, variant_name, field_name
                                ));
                            match field_token {
                                FieldIdToken::Ident(ident) => ident.clone(),
                                FieldIdToken::Literal(lit) => Ident2::new(
                                    &format!("f{lit}"),
                                    Span2::call_site()
                                ),
                            }
                        })
                        .collect();
                    quote! {
                        writeln!(
                            f,
                            #trimmed_variant_docstr,
                            #(& #variant_docstr_fields),*
                        )?;
                    }
                })
                .chain([
                    // Write an empty line between original msg & ErrorCtx,
                    // but only perform the writeln!() call if the consumer
                    // crate is built with the build feature enabled, or
                    // without using the `feature` attribute argument:
                    if let Some(bf) = build_feature {
                        assert_eq!(
                            bf.name,
                            Ident2::new("feature", Span2::call_site())
                        );
                        let feature = &bf.value;
                        quote! {
                            #[cfg(feature = #feature)]
                            writeln!(f, "")?;
                            #[cfg(feature = #feature)]
                            writeln!(f, "{}", &ctx)?;
                        }
                    } else {
                        quote! {
                            writeln!(f, "")?;
                            writeln!(f, "{}", &ctx)?;
                        }
                    }
                ])
                .collect();

            if let Some(bf) = build_feature {
                let feature = &bf.value;
                quote! {
                    #[cfg(feature = #feature)]
                    Self :: #variant_name  #vbind_list_with_ctx  => {
                        #( #vdocstr_writelns )*
                    },

                    #[cfg(not(feature = #feature))]
                    Self :: #variant_name  #vbind_list  => {
                        #( #vdocstr_writelns )*
                    },
                }
            } else {
                quote! {
                    Self :: #variant_name  #vbind_list_with_ctx  => {
                        #( #vdocstr_writelns )*
                    },
                }
            }

        })
        .collect();

    quote! {
        match self {
            #( #variant_writelns )*
        }
    }
}

fn get_docstrs_from_attrs(attrs: &[Attribute]) -> Vec<String> {
    attrs.iter()
        .filter(|Attribute { meta, .. }| {
            let Meta::NameValue(v) = meta else { return false };
            let segs = v.path.segments.iter().collect::<Vec<_>>();
            let [PathSegment { ident, arguments: PathArguments::None }] = &*segs
            else { return false };
            ident.to_string() == "doc"
        })
        .map(|Attribute { meta, .. }|  {
            let Meta::NameValue(v) = meta else { unreachable!() };
            let Expr::Lit(ExprLit { lit, .. }) = &v.value else { unreachable!() };
            let Lit::Str(s) = lit else { unreachable!() };
            s.token().to_string()
                .trim_start_matches(r#"" "#)
                .trim_end_matches(r#"""#)
                .to_string()
        })
        .collect()
}

/// Return a modified docstring that has each site where
/// a field is mentioned (e.g. {foo}) replaced by a {}.
fn modify_docstr(
    docstr: &str,
    fields_sites: &[(Range<usize>, &str)],
) -> String {
    fields_sites.iter()
        .rev(/*replace from last match to first*/)
        .fold(docstr.to_string(), |docstr, &(Range { start, end }, _)| {
            const EMPTY_FMT_STR: &str = "{}";
            let   preamble = &docstr[..start];
            let  postamble = &docstr[end..];
            preamble.to_string() + EMPTY_FMT_STR + postamble
        })
}

/// Find the struct/enum fields used in a docstring, and return
/// them in the order thay they occur in within the string.
fn find_docstring_fields(docstr: &str) -> Vec<(Range<usize>, &str)> {
    static FIELD_SPECIFIER_REGEX: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"\{[A-Za-z0-9_]+\}").unwrap()
    });
    FIELD_SPECIFIER_REGEX.find_iter(docstr)
        .map(|m: Match| {
            let field_name = m.as_str()
                .strip_prefix('{').unwrap()
                .strip_suffix('}').unwrap();
            (m.range(), field_name)
        })
        .collect()
}

fn writeln_for_ErrorCtx_field(
    build_feature: Option<&BuildFeatureAttr>,
    fields: &Fields,
) -> TokenStream2 {
    match &fields {
        Fields::Named(_) | Fields::Unit => {
            let ident = Ident2::new("ctx", Span2::call_site());
            let err_ctx_field = FieldIdToken::Ident(ident);
            if let Some(BuildFeatureAttr { name, value, .. }) = build_feature {
                assert_eq!(name, &Ident2::new("feature", Span2::call_site()));
                // Write the error ctx, but only perform the writeln!() call if
                // the consumer crate is built with the build feature enabled:
                quote! {
                    #[cfg(feature = #value)]
                    writeln!(f, "{}", &self . #err_ctx_field)?;
                }
            } else {
                // Write the error ctx
                quote! {
                    writeln!(f, "{}", &self . #err_ctx_field)?;
                }
            }
        }
        Fields::Unnamed(u) => {
            let field_num = u.unnamed.iter()
                .enumerate()
                .last()
                .map(|(field_num, _field)| field_num)
                .unwrap_or_default();
            let lit = format!("{}", field_num + 1);
            let lit = LitInt::new(&lit, Span2::call_site());
            let err_ctx_field = FieldIdToken::Literal(lit);
            if let Some(BuildFeatureAttr { name, value, .. }) = build_feature {
                assert_eq!(name, &Ident2::new("feature", Span2::call_site()));
                // Write the error ctx, but only perform the writeln!() call if
                // the consumer crate is built with the build feature enabled:
                quote! {
                    #[cfg(feature = #value)]
                    writeln!(f, "{}", &self . #err_ctx_field)?;
                }
            } else {
                // Write the error ctx
                quote! {
                    writeln!(f, "{}", &self . #err_ctx_field)?;
                }
            }
        }
    }
}

/// A quote!()-injectable token representing one of:
/// - a name (for named fields), or
/// - a number (for unnamd fields)
#[derive(Debug)]
enum FieldIdToken {
    Ident(Ident2),
    Literal(LitInt),
}

impl quote::ToTokens for FieldIdToken {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let ts = match self {
            Self::Ident(ident) => quote! { #ident },
            Self::Literal(literal) => quote! { #literal },
        };
        tokens.extend(ts);
    }
}

enum FieldMap {
    Struct(HashMap<String, FieldIdToken>),
    Enum(HashMap<Ident2, HashMap<String, FieldIdToken>>), // for each variant
}

fn single_field_mapping(
    ident: Option<Ident2>,
    num: Option<usize>,
) -> (String, FieldIdToken) {
    match (ident, num) {
        (Some(ident), _) => {
            let field_name = format!("{ident}");
            (field_name, FieldIdToken::Ident(ident.clone()))
        }
        (_, Some(num)) => {
            let field_num = format!("{num}");
            let lit = LitInt::new(&field_num, Span2::call_site());
            (field_num, FieldIdToken::Literal(lit))
        },
        _ => unreachable!(),
    }
}

fn create_fields_map(fields: &Fields) -> HashMap<String, FieldIdToken> {
    match fields {
        Fields::Named(n) => n.named.iter()
            .map(|Field { ident, .. }| {
                single_field_mapping(ident.clone(), None)
            })
            .chain({ // ErrorCtx field
                let ident = Ident2::new("ctx", Span2::call_site());
                [ single_field_mapping(Some(ident), None) ]
            })
            .collect(),
        Fields::Unit => std::iter::empty()
            .chain({ // ErrorCtx field
                let ident = Ident2::new("ctx", Span2::call_site());
                [ single_field_mapping(Some(ident), None) ]
            })
            .collect(),
        Fields::Unnamed(u) => {
            let mut fields: Vec<(String, FieldIdToken)> = u.unnamed.iter()
                .enumerate()
                .map(|(i, Field { ident, .. })| {
                    assert!(ident.is_none());
                    single_field_mapping(None, Some(i))
                })
                .collect();
            fields.push({ // ErrorCtx field
                single_field_mapping(None, Some(fields.len()))
            });
            fields.into_iter().collect()
        },
    }
}
