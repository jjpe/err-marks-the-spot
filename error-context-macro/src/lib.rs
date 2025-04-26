//!

use proc_macro::{TokenStream, Span};
use proc_macro2::{TokenStream as TokenStream2, Span as Span2};
use quote::quote;
use syn::{parse_macro_input, DeriveInput};


#[proc_macro_attribute]
pub fn contextual_error(attr: TokenStream, item: TokenStream) -> TokenStream {
    // let DeriveInput {
    //     attrs: attr_attrs,
    //     vis: attr_vis,
    //     ident: attr_ident,
    //     generics: attr_generics,
    //     data: attr_data
    // } = parse_macro_input!(attr as DeriveInput);
    let DeriveInput {
        attrs: item_attrs,
        vis: item_vis,
        ident: item_ident,
        generics: item_generics,
        data: item_data
    } = parse_macro_input!(item as DeriveInput);

    println!("attr: {attr:#?}");
    println!("attrs: {item_attrs:#?}");



    let output: TokenStream2 = quote! {
        // TODO
    };
    TokenStream::from(output)
}
