pub fn main() {
    let module =
        include_bytes!("../../../../target/wasm32-unknown-unknown/debug/wavedash_example.wasm");
    wavedash_host::Wasm::new(module).run();
}
