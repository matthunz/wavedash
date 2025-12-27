use bevy::{prelude::*, ptr::Ptr};
use serde::Serialize;
use std::{
    any::TypeId,
    collections::HashMap,
    ffi::CStr,
    mem::{self, MaybeUninit},
    slice,
    sync::Arc,
};
use wasmtime::{Caller, Engine, Linker, Memory, Module, Store, TypedFunc};
use wavedash_core::Buffer;

pub struct Context {
    memory: MaybeUninit<Memory>,
    alloc_fn: MaybeUninit<TypedFunc<(i32, i32), i32>>,
    resources: HashMap<TypeId, ResourceFactory>,
}

impl Context {
    pub fn memory(&self) -> Memory {
        unsafe { self.memory.assume_init() }
    }

    pub fn alloc_fn(&self) -> TypedFunc<(i32, i32), i32> {
        unsafe { self.alloc_fn.assume_init_ref() }.clone()
    }
}

pub struct Wasm {
    main_fn: TypedFunc<i64, ()>,
    store: Store<Context>,
}

impl Wasm {
    pub fn new(module: &[u8]) -> Self {
        let engine = Engine::default();
        let module = Module::new(&engine, &module).unwrap();

        let cx = Context {
            memory: MaybeUninit::uninit(),
            alloc_fn: MaybeUninit::uninit(),
            resources: HashMap::new(),
        };
        let mut store = Store::new(&engine, cx);

        let mut linker = Linker::new(&engine);
        linker
            .func_wrap(
                "__wavedash__",
                "__wavedash_get_resource",
                get_resource_handler,
            )
            .unwrap();

        let instance = linker.instantiate(&mut store, &module).unwrap();

        let memory = instance
            .get_memory(&mut store, "memory")
            .expect("WASM module should export memory");
        store.data_mut().memory.write(memory);

        let alloc_fn = instance
            .get_typed_func::<(i32, i32), i32>(&mut store, "__wavedash_alloc")
            .unwrap();
        store.data_mut().alloc_fn.write(alloc_fn);

        let main_fn = instance
            .get_typed_func::<i64, ()>(&mut store, "__wavedash_main")
            .unwrap();

        Self { main_fn, store }
    }

    pub fn run(&mut self, world: &mut World) {
        self.main_fn
            .call(&mut self.store, world as *mut World as i64)
            .unwrap();
    }

    pub fn insert_resource<R>(&mut self)
    where
        R: 'static + Serialize,
    {
        self.store.data_mut().resources.insert(
            TypeId::of::<R>(),
            ResourceFactory {
                serialize_fn: Arc::new(|ptr: Ptr| unsafe {
                    let r: &R = &*(ptr.as_ptr() as *const R);
                    bincode::serialize(r).unwrap_or_default()
                }),
            },
        );
    }
}

#[derive(Clone)]
struct ResourceFactory {
    serialize_fn: Arc<dyn Fn(Ptr) -> Vec<u8> + Send + Sync>,
}

fn get_resource_handler(
    mut caller: Caller<Context>,
    result_ptr: i32,
    world_ptr: i64,
    type_path_ptr: i32,
    type_path_len: i32,
) {
    let memory = caller.data().memory();

    let type_path_bytes = unsafe {
        slice::from_raw_parts(
            memory.data_ptr(&caller).add(type_path_ptr as usize),
            type_path_len as usize,
        )
    };
    let type_path_str = CStr::from_bytes_with_nul(type_path_bytes)
        .unwrap()
        .to_str()
        .unwrap();

    dbg!(type_path_str);

    let world = unsafe { &mut *(world_ptr as *mut World) };

    let registry = world.get_resource::<AppTypeRegistry>().unwrap();
    let registry_ref = registry.read();
    let type_registration = registry_ref.get_with_type_path(&type_path_str).unwrap();
    let type_id = type_registration.type_id();
    drop(registry_ref);

    let resource_id = world.components().get_resource_id(type_id).unwrap();

    let ptr = world.get_resource_by_id(resource_id).unwrap();
    let buf = (caller.data().resources.get(&type_id).unwrap().serialize_fn)(ptr);

    let alloc_fn = caller.data().alloc_fn();
    let allocated_ptr = alloc_fn
        .call(
            &mut caller,
            (buf.len() as i32, mem::align_of::<Buffer>() as _),
        )
        .unwrap();

    memory.data_mut(&mut caller)[allocated_ptr as usize..allocated_ptr as usize + buf.len()]
        .copy_from_slice(&buf);

    let result_ptr = unsafe { memory.data_ptr(&caller).add(result_ptr as _) };
    unsafe {
        *(result_ptr as *mut Buffer) = Buffer {
            ptr: allocated_ptr,
            len: buf.len() as i32,
        };
    }
}
