//!

use proc_macro::{
    token_stream::IntoIter,
    Delimiter, TokenStream, TokenTree,
};
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
pub fn err_marks_the_spot(attr: TokenStream, item: TokenStream) -> TokenStream {
    let DeriveInput {
        attrs: item_attrs,
        vis: item_vis,
        ident: item_ident,
        generics: item_generics,
        data: item_data
    } = parse_macro_input!(item as DeriveInput);
    let fattrs = FieldAttrs::parse(attr);
    let field_attrs = fattrs.field_attr_vec();
    let ctor_attrs = fattrs.ctor_attr_vec();
    let ctor_impl_block = generate_ctor_impl_block(
        &ctor_attrs,
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
                        ident: Ident2::new("err_marks_the_spot", Span2::call_site()),
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
struct FieldAttrs {
    build_flag: Option<FeatureFlagAttr>,
    inline_ctors: Option<InlineCtorsAttr>,
}

impl FieldAttrs {
    fn parse(attr: TokenStream) -> Self {
        let mut attr_iter = attr.into_iter().peekable();
        let mut field_attrs = Self {
            build_flag: None,
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
                    let arg = FeatureFlagAttr::parse(&mut attr_iter);
                    field_attrs.build_flag = Some(arg);
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
        if let Some(build_flag) = &self.build_flag {
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
#[derive(Debug)]
struct FeatureFlagAttr {
    #[allow(unused)]
    name: Ident2,
    name_stream: TokenStream2,
    value: Expr,
}

impl FeatureFlagAttr {
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
                    let mut token_stream = self.name_stream.clone();
                    token_stream.extend(TokenStream2::from(
                        TokenTree2::Punct(Punct2::new('=', Spacing2::Alone))
                    ));
                    token_stream.extend(self.value.to_token_stream());
                    token_stream
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
        attr_arg_name: &str
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
            )
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
