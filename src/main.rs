use bevy::ecs::world::World;
use std::fs;
use wasmer::{
    imports, AsStoreRef, Function, FunctionEnv, FunctionEnvMut, Instance, Memory, MemoryView,
    Module, Store, TypedFunction, WasmPtr,
};

pub struct Env {
    memory: Option<Memory>,
    world: *mut World,
}

unsafe impl Send for Env {}

unsafe impl Sync for Env {}

impl Env {
    fn set_memory(&mut self, memory: Memory) {
        self.memory = Some(memory);
    }

    fn get_memory(&self) -> &Memory {
        self.memory.as_ref().unwrap()
    }

    fn view<'a>(&'a self, store: &'a impl AsStoreRef) -> MemoryView<'a> {
        self.get_memory().view(store)
    }
}

fn log(ctx: FunctionEnvMut<Env>, msg: u32, msg_len: u32) {
    let view = ctx.data().view(&ctx);
    let s = WasmPtr::<u8>::new(msg)
        .read_utf8_string(&view, msg_len)
        .unwrap();
    dbg!(s);
}

fn world_resource(ctx: FunctionEnvMut<Env>, msg: u32, msg_len: u32) -> u32 {
    let view = ctx.data().view(&ctx);
    let s = WasmPtr::<u8>::new(msg)
        .read_utf8_string(&view, msg_len)
        .unwrap();
    dbg!(s);
    0
}

fn main() {
    let mut store = Store::default();

    let wasm = fs::read("target/wasm32-unknown-unknown/debug/example.wasm").unwrap();
    let module = Module::new(&store, wasm).unwrap();

    let mut world = World::new();

    let env = FunctionEnv::new(
        &mut store,
        Env {
            memory: None,
            world: &mut world as _,
        },
    );
    let log_fn = Function::new_typed_with_env(&mut store, &env, log);
    let world_fn = Function::new_typed_with_env(&mut store, &env, world_resource);
    let import_object = imports! {
        "wavedash" => {
            "_log" => log_fn,
            "_world_resource" => world_fn,
        }
    };

    let instance = Instance::new(&mut store, &module, &import_object).unwrap();
    let memory = instance.exports.get_memory("memory").unwrap();
    env.as_mut(&mut store).set_memory(memory.clone());

    let main: TypedFunction<(), ()> = instance
        .exports
        .get_function("main")
        .unwrap()
        .typed(&mut store)
        .unwrap();
    main.call(&mut store).unwrap();
}
