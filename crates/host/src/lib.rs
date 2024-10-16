use bevy::{
    prelude::*,
    ptr::{Ptr, PtrMut},
    utils::HashMap,
};
use serde::{de::DeserializeOwned, Serialize};
use std::{any::TypeId, ffi::CString, sync::Arc};
use wasmer::{
    imports, Function, FunctionEnv, FunctionEnvMut, Instance, Memory, MemoryType, MemoryView,
    Module, Store, Value, WasmPtr,
};
use wavedash_core::{Request, Response};

#[derive(Default)]
pub struct WavedashPlugin {
    module: Vec<u8>,
    resources: HashMap<
        TypeId,
        (
            Arc<dyn Fn(Ptr) -> serde_json::Value + Send + Sync>,
            Arc<dyn Fn(PtrMut, serde_json::Value) + Send + Sync>,
        ),
    >,
}

impl WavedashPlugin {
    pub fn new(module: Vec<u8>) -> Self {
        Self {
            module,
            resources: HashMap::new(),
        }
    }

    pub fn with_resource<R>(mut self) -> Self
    where
        R: Resource + DeserializeOwned + Serialize,
    {
        self.resources.insert(
            TypeId::of::<R>(),
            (
                Arc::new(|ptr| {
                    let r: &R = unsafe { ptr.deref() };
                    serde_json::to_value(r).unwrap()
                }),
                Arc::new(|ptr, value| {
                    let r: &mut R = unsafe { ptr.deref_mut() };
                    *r = serde_json::from_value(value).unwrap();
                }),
            ),
        );
        self
    }
}

impl Plugin for WavedashPlugin {
    fn build(&self, app: &mut App) {
        let mut store = Store::default();
        let module = Module::new(&store, &self.module).unwrap();

        let env = FunctionEnv::new(
            &mut store,
            Env {
                memory: None,
                func: None,
                world: app.world_mut() as _,
                resources: self.resources.clone(),
            },
        );
        let memory = Memory::new(&mut store, MemoryType::new(1, None, false)).unwrap();
        let import_object = imports! {
            "__wbindgen_placeholder__" => {
                "__wbindgen_describe" => Function::new_typed(&mut store, |_: u32| {}),
                "__wbindgen_throw" => Function::new_typed(&mut store, |_: i32, _: i32| {}),
                "memory" => memory,
            },
            "__wbindgen_externref_xform__" => {
                "__wbindgen_externref_table_grow" => Function::new_typed(&mut store, |_delta: i32| 0i32),
                "__wbindgen_externref_table_set_null" => Function::new_typed(&mut store, |_idx: i32| {}),
            },
            "__wavedash__" => {
                "__wavedash_request" => Function::new_typed_with_env(&mut store, &env, request),
            }
        };
        let instance = Instance::new(&mut store, &module, &import_object).unwrap();

        let memory = instance.exports.get_memory("memory").unwrap();
        env.as_mut(&mut store).memory = Some(memory.clone());
        env.as_mut(&mut store).func = Some(
            instance
                .exports
                .get_function("__wavedash_alloc")
                .unwrap()
                .clone(),
        );

        let run_fn = instance.exports.get_function("run").unwrap();
        run_fn.call(&mut store, &[]).unwrap();

        let run_fn = instance
            .exports
            .get_function("__wavedash_run_system")
            .unwrap()
            .clone();

        app.insert_resource(WasmRegistry { store, run_fn, env })
            .add_systems(Update, run);
    }
}

#[derive(Resource)]
struct WasmRegistry {
    store: Store,
    run_fn: Function,
    env: FunctionEnv<Env>,
}

fn run(world: &mut World) {
    let world_ptr = world as *mut _;
    let registry = &mut *world.get_resource_mut::<WasmRegistry>().unwrap();
    registry.env.as_mut(&mut registry.store).world = world_ptr;
    registry
        .run_fn
        .clone()
        .call(&mut registry.store, &[Value::I32(0)])
        .unwrap();
}

pub struct Env {
    memory: Option<Memory>,
    func: Option<Function>,
    world: *mut World,
    resources: HashMap<
        TypeId,
        (
            Arc<dyn Fn(Ptr) -> serde_json::Value + Send + Sync>,
            Arc<dyn Fn(PtrMut, serde_json::Value) + Send + Sync>,
        ),
    >,
}

unsafe impl Send for Env {}

unsafe impl Sync for Env {}

pub fn read_string(view: &MemoryView, start: u32, len: u32) -> anyhow::Result<String> {
    Ok(WasmPtr::<u8>::new(start).read_utf8_string(view, len)?)
}

fn request(mut ctx: FunctionEnvMut<Env>, input_ptr: u32, input_len: u32) -> u32 {
    let (data, mut store) = ctx.data_and_store_mut();

    let view = data.memory.as_ref().unwrap().view(&store);
    let input = read_string(&view, input_ptr, input_len).unwrap();
    let req: Request = serde_json::from_str(&input).unwrap();

    let res = match req {
        Request::Log(s) => {
            println!("{}", s);
            Response::Empty
        }
        Request::GetResource { type_path } => {
            let world = unsafe { &mut *data.world };
            let registry = world.get_resource::<AppTypeRegistry>().unwrap();
            let registry_ref = registry.read();
            let type_registration = registry_ref.get_with_type_path(&type_path).unwrap();

            let component_id = world
                .components()
                .get_resource_id(type_registration.type_id())
                .unwrap();
            let ptr = world.get_resource_by_id(component_id).unwrap();

            let json = data.resources.get(&type_registration.type_id()).unwrap().0(ptr);

            Response::Resource(json)
        }
        Request::SetResource { type_path, value } => {
            let world = unsafe { &mut *data.world };
            let registry = world.get_resource::<AppTypeRegistry>().unwrap();
            let registry_ref = registry.read();
            let type_registration = registry_ref.get_with_type_path(&type_path).unwrap();
            let type_id = type_registration.type_id();
            let component_id = world
                .components()
                .get_resource_id(type_registration.type_id())
                .unwrap();
            drop(registry_ref);

            let mut ptr = world.get_resource_mut_by_id(component_id).unwrap();

            data.resources.get(&type_id).unwrap().1(
                ptr.as_mut(),
                serde_json::from_value(value).unwrap(),
            );

            Response::Empty
        }
    };

    let json = serde_json::to_string(&res).unwrap();
    let buf = CString::new(json).unwrap().into_bytes_with_nul();

    let values = data
        .func
        .as_ref()
        .unwrap()
        .call(&mut store, &[Value::I32(buf.len() as i32)])
        .unwrap();

    let view = data.memory.as_ref().unwrap().view(&store);

    let ptr = values[0].i32().unwrap();
    view.write(ptr as _, &buf).unwrap();

    ptr as u32
}
