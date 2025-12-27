use proc_macro::TokenStream;
use quote::quote;

#[proc_macro_attribute]
pub fn main(input: TokenStream, attrs: TokenStream) -> TokenStream {
    let input = proc_macro2::TokenStream::from(input);
    let attrs = proc_macro2::TokenStream::from(attrs);

    quote! {
        #attrs
        #input

        #[unsafe(no_mangle)]
        extern "C" fn __wavedash_main() {
            main()
        }
    }
    .into()
}
