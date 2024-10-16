use proc_macro::TokenStream;
use quote::quote;

#[proc_macro_attribute]
pub fn main(input: TokenStream, attrs: TokenStream) -> TokenStream {
    let input = proc_macro2::TokenStream::from(input);
    let attrs = proc_macro2::TokenStream::from(attrs);

    quote! {
        #[no_mangle]
        extern "C" fn run() {
            #input
            #attrs
            main()
        }
    }
    .into()
}
