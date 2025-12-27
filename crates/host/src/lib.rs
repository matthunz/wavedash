use std::{mem::MaybeUninit, slice};
use wasmtime::{Caller, Engine, Linker, Memory, MemoryType, Module, Store, TypedFunc};

pub struct Context {
    memory: MaybeUninit<Memory>,
}

impl Context {
    pub fn memory(&self) -> &Memory {
        unsafe { &*self.memory.as_ptr() }
    }
}

pub fn store(engine: &Engine) -> Store<Context> {
    let cx = Context {
        memory: MaybeUninit::uninit(),
    };
    let mut store = Store::new(engine, cx);

    let memory = Memory::new(&mut store, MemoryType::new(1, None)).unwrap();
    store.data_mut().memory = MaybeUninit::new(memory);
    store
}

pub struct Wasm {
    main_fn: TypedFunc<(), ()>,
    store: Store<Context>,
}

impl Wasm {
    pub fn new(module: &[u8]) -> Self {
        let engine = Engine::default();
        let module = Module::new(&engine, &module).unwrap();

        let mut store = store(&engine);
        let memory = *store.data().memory();

        let mut linker = Linker::new(&engine);
        link(&mut linker, &mut store, memory);

        let instance = linker.instantiate(&mut store, &module).unwrap();

        let main_fn = instance
            .get_typed_func::<(), ()>(&mut store, "__wavedash_main")
            .unwrap();

        Self { main_fn, store }
    }

    pub fn run(&mut self) {
        self.main_fn.call(&mut self.store, ()).unwrap();
    }
}

pub fn link(linker: &mut Linker<Context>, store: &mut Store<Context>, memory: Memory) {
    linker.define(store, "env", "memory", memory).unwrap();
    linker
        .func_wrap("__wavedash__", "__wavedash_log", log_handler)
        .unwrap();
}

fn log_handler(caller: Caller<Context>, msg_ptr: i32, msg_len: i32) {
    let memory = caller.data().memory();
    let view = unsafe {
        slice::from_raw_parts(
            memory.data_ptr(&caller).add(msg_ptr as usize),
            msg_len as usize,
        )
    };

    let msg = String::from_utf8(view.to_vec()).unwrap();
    println!("{}", msg);
}
