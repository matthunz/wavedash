use proc_macro::TokenStream;
use quote::quote;

#[proc_macro_attribute]
pub fn main(input: TokenStream, attrs: TokenStream) -> TokenStream {
    let input = proc_macro2::TokenStream::from(input);
    let attrs = proc_macro2::TokenStream::from(attrs);

    quote! {
        #[no_mangle]
        extern "C" fn __wavedash_main() {
            #attrs
            #input


            fn __wavedash_runner<Marker, F>(mut system: F)
            where
                F: wavedash::WasmSystemParamFunction<Marker> + 'static,
            {
                let mut __wavedash_world = unsafe { wavedash::World::current() };
                let param = unsafe {
                    use wavedash::WasmSystemParam;
                    F::Params::from_wasm_world(&mut __wavedash_world)
                };
                system.run(param);
            }

            __wavedash_runner(main)
            
        }
    }
    .into()
}
