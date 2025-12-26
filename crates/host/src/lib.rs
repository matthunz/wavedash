use bevy::{
    ecs::{
        component::{Mutable, StorageType},
        lifecycle::ComponentHook,
    },
    platform::collections::HashMap,
    prelude::*,
    ptr::{Ptr, PtrMut},
};
use serde::{de::DeserializeOwned, Serialize};
use std::{any::TypeId, cell::RefCell, sync::Arc};
use wasmtime::{Engine, Instance, Linker, Memory, Module, Store, TypedFunc};

thread_local! {
    static WORLD_PTR: RefCell<*mut World> = RefCell::new(std::ptr::null_mut());
    static RESOURCES: RefCell<HashMap<TypeId, ResourceFactory>> = RefCell::new(HashMap::new());
}

#[derive(Clone)]
struct ResourceFactory {
    serialize_fn: Arc<dyn Fn(Ptr) -> serde_json::Value + Send + Sync>,
    deserialize_fn: Arc<dyn Fn(PtrMut, serde_json::Value) + Send + Sync>,
}

pub struct WasmModule {
    store: Store<()>,
    main_fn: TypedFunc<(), ()>,
    #[allow(dead_code)]
    instance: Instance,
    #[allow(dead_code)]
    memory: Memory,
    resources: HashMap<TypeId, ResourceFactory>,
}

impl WasmModule {
    pub fn new(module: Vec<u8>) -> Self {
        let engine = Engine::default();
        let module = Module::new(&engine, &module).unwrap();

        let mut store = Store::new(&engine, ());
        let mut linker = Linker::new(&engine);

        let memory = Memory::new(&mut store, wasmtime::MemoryType::new(1, None)).unwrap();
        linker.define(&mut store, "env", "memory", memory).unwrap();

        linker
            .func_wrap(
                "__wavedash__",
                "__wavedash_log",
                |mut caller: wasmtime::Caller<'_, ()>, msg_ptr: i32, msg_len: i32| -> i32 {
                    log_handler(&mut caller, msg_ptr, msg_len).unwrap();
                    0
                },
            )
            .unwrap();

        linker
            .func_wrap(
                "__wavedash__",
                "__wavedash_get_resource",
                |mut caller: wasmtime::Caller<'_, ()>, type_path_ptr: i32, type_path_len: i32| {
                    get_resource_handler(&mut caller, type_path_ptr, type_path_len).unwrap()
                },
            )
            .unwrap();

        linker
            .func_wrap(
                "__wavedash__",
                "__wavedash_set_resource",
                |mut caller: wasmtime::Caller<'_, ()>,
                 type_path_ptr: i32,
                 type_path_len: i32,
                 value_ptr: i32,
                 value_len: i32|
                 -> i32 {
                    set_resource_handler(
                        &mut caller,
                        type_path_ptr,
                        type_path_len,
                        value_ptr,
                        value_len,
                    )
                    .unwrap();
                    0
                },
            )
            .unwrap();

        let instance = linker.instantiate(&mut store, &module).unwrap();

        let main_fn = instance
            .get_typed_func::<(), ()>(&mut store, "__wavedash_main")
            .unwrap();

        Self {
            store,
            main_fn,
            instance,
            memory,
            resources: HashMap::new(),
        }
    }

    pub fn with_resource<R>(mut self) -> Self
    where
        R: Resource + DeserializeOwned + Serialize,
    {
        self.resources.insert(
            TypeId::of::<R>(),
            ResourceFactory {
                serialize_fn: Arc::new(|ptr| {
                    let r: &R = unsafe { ptr.deref() };
                    serde_json::to_value(r).unwrap()
                }),
                deserialize_fn: Arc::new(|ptr, value| {
                    let r: &mut R = unsafe { ptr.deref_mut() };
                    *r = serde_json::from_value(value).unwrap();
                }),
            },
        );
        self
    }
}

impl Component for WasmModule {
    const STORAGE_TYPE: StorageType = StorageType::SparseSet;

    type Mutability = Mutable;

    fn on_insert() -> Option<ComponentHook> {
        Some(|mut world, cx| {
            let world_ref = unsafe { world.as_unsafe_world_cell().world_mut() };
            let world_ptr = world_ref as *mut _;

            let wasm = &mut *world.get_mut::<WasmModule>(cx.entity).unwrap();

            WORLD_PTR.with(|w| {
                *w.borrow_mut() = world_ptr;
            });
            RESOURCES.with(|r| {
                *r.borrow_mut() = wasm.resources.clone();
            });

            wasm.main_fn.call(&mut wasm.store, ()).unwrap();
        })
    }
}

pub fn read_string(
    memory: &Memory,
    store: &Store<()>,
    start: u32,
    len: u32,
) -> anyhow::Result<String> {
    let data = memory.data(store);
    let slice = &data[start as usize..][..len as usize];
    Ok(String::from_utf8(slice.to_vec())?)
}

fn log_handler(
    caller: &mut wasmtime::Caller<'_, ()>,
    msg_ptr: i32,
    msg_len: i32,
) -> anyhow::Result<()> {
    let memory = caller
        .get_export("memory")
        .and_then(|e| e.into_memory())
        .ok_or_else(|| anyhow::anyhow!("memory not found"))?;

    let view = unsafe {
        std::slice::from_raw_parts(
            memory.data_ptr(&*caller).add(msg_ptr as usize),
            msg_len as usize,
        )
    };
    let msg = String::from_utf8(view.to_vec())?;
    println!("{}", msg);

    Ok(())
}

fn get_resource_handler(
    caller: &mut wasmtime::Caller<'_, ()>,
    type_path_ptr: i32,
    type_path_len: i32,
) -> anyhow::Result<i32> {
    let memory = caller
        .get_export("memory")
        .and_then(|e| e.into_memory())
        .ok_or_else(|| anyhow::anyhow!("memory not found"))?;

    let view = unsafe {
        std::slice::from_raw_parts(
            memory.data_ptr(&*caller).add(type_path_ptr as usize),
            type_path_len as usize,
        )
    };
    let type_path = String::from_utf8(view.to_vec())?;

    let world_ptr = WORLD_PTR.with(|w| *w.borrow());
    let resources = RESOURCES.with(|r| r.borrow().clone());
    let world = unsafe { &mut *world_ptr };

    let registry = world
        .get_resource::<AppTypeRegistry>()
        .ok_or_else(|| anyhow::anyhow!("AppTypeRegistry not found"))?;
    let registry_ref = registry.read();
    let type_registration = registry_ref
        .get_with_type_path(&type_path)
        .ok_or_else(|| anyhow::anyhow!("type not found: {}", type_path))?;

    let component_id = world
        .components()
        .get_resource_id(type_registration.type_id())
        .ok_or_else(|| anyhow::anyhow!("component_id not found"))?;
    let ptr = world
        .get_resource_by_id(component_id)
        .ok_or_else(|| anyhow::anyhow!("resource not found"))?;

    let json = (resources
        .get(&type_registration.type_id())
        .ok_or_else(|| anyhow::anyhow!("resource factory not found"))?
        .serialize_fn)(ptr);

    let json_str = serde_json::to_string(&json)?;
    let mut buf = json_str.into_bytes();
    buf.push(0); // Add null terminator

    // Get the alloc function and call it to allocate space
    let alloc_fn = caller
        .get_export("__wavedash_alloc")
        .and_then(|e| e.into_func())
        .ok_or_else(|| anyhow::anyhow!("__wavedash_alloc not found"))?;

    let alloc_typed: TypedFunc<i32, i32> = alloc_fn.typed(&*caller)?;
    let ptr = alloc_typed.call(&mut *caller, buf.len() as i32)?;

    // Write the response to the allocated memory
    let memory = caller
        .get_export("memory")
        .and_then(|e| e.into_memory())
        .ok_or_else(|| anyhow::anyhow!("memory not found"))?;

    let data_mut = memory.data_mut(&mut *caller);
    data_mut[ptr as usize..ptr as usize + buf.len()].copy_from_slice(&buf);

    Ok(ptr)
}

fn set_resource_handler(
    caller: &mut wasmtime::Caller<'_, ()>,
    type_path_ptr: i32,
    type_path_len: i32,
    value_ptr: i32,
    value_len: i32,
) -> anyhow::Result<()> {
    let memory = caller
        .get_export("memory")
        .and_then(|e| e.into_memory())
        .ok_or_else(|| anyhow::anyhow!("memory not found"))?;

    let view = unsafe {
        std::slice::from_raw_parts(
            memory.data_ptr(&*caller).add(type_path_ptr as usize),
            type_path_len as usize,
        )
    };
    let type_path = String::from_utf8(view.to_vec())?;

    let view = unsafe {
        std::slice::from_raw_parts(
            memory.data_ptr(&*caller).add(value_ptr as usize),
            value_len as usize,
        )
    };
    let value_data = String::from_utf8(view.to_vec())?;

    let value: serde_json::Value = serde_json::from_str(&value_data)?;

    let world_ptr = WORLD_PTR.with(|w| *w.borrow());
    let resources = RESOURCES.with(|r| r.borrow().clone());
    let world = unsafe { &mut *world_ptr };

    let registry = world
        .get_resource::<AppTypeRegistry>()
        .ok_or_else(|| anyhow::anyhow!("AppTypeRegistry not found"))?;
    let registry_ref = registry.read();
    let type_registration = registry_ref
        .get_with_type_path(&type_path)
        .ok_or_else(|| anyhow::anyhow!("type not found: {}", type_path))?;
    let type_id = type_registration.type_id();
    let component_id = world
        .components()
        .get_resource_id(type_registration.type_id())
        .ok_or_else(|| anyhow::anyhow!("component_id not found"))?;
    drop(registry_ref);

    let mut ptr = world
        .get_resource_mut_by_id(component_id)
        .ok_or_else(|| anyhow::anyhow!("resource not found"))?;

    (resources
        .get(&type_id)
        .ok_or_else(|| anyhow::anyhow!("resource factory not found"))?
        .deserialize_fn)(ptr.as_mut(), value);

    Ok(())
}
