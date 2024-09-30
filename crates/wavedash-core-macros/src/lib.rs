use proc_macro::TokenStream;
use quote::quote;
use syn::{self, ItemStruct};

#[proc_macro_derive(Named)]
pub fn named(input: TokenStream) -> TokenStream {
    let ast: ItemStruct = syn::parse(input).unwrap();

    let name = &ast.ident;
    let gen = quote! {
        impl wavedash_core::Named for #name {
            fn name() -> std::borrow::Cow<'static, str> {
                std::borrow::Cow::Borrowed(stringify!(#name))
            }
        }
    };
    gen.into()
}
