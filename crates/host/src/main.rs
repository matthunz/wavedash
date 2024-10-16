use bevy::prelude::*;
use std::ffi::CString;
use wasmer::{
    imports, Function, FunctionEnv, FunctionEnvMut, Instance, Memory, MemoryType, MemoryView,
    Module, Store, Value, WasmPtr,
};
use wavedash_core::{ExampleResource, Request, Response};

fn main() -> anyhow::Result<()> {
    let mut store = Store::default();
    let module = Module::new(
        &store,
        include_bytes!("../../../target/wasm32-unknown-unknown/debug/wavedash_example.wasm"),
    )?;

    let mut world = World::new();
    let registry = AppTypeRegistry::default();
    registry.write().register::<ExampleResource>();
    world.insert_resource(registry);
    world.insert_resource(ExampleResource { value: 42 });

    let env = FunctionEnv::new(
        &mut store,
        Env {
            memory: None,
            func: None,
            world,
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
    let instance = Instance::new(&mut store, &module, &import_object)?;

    let memory = instance.exports.get_memory("memory")?;
    env.as_mut(&mut store).memory = Some(memory.clone());
    env.as_mut(&mut store).func = Some(instance.exports.get_function("__wavedash_alloc")?.clone());

    let run_fn = instance.exports.get_function("run")?;
    run_fn.call(&mut store, &[])?;

    Ok(())
}

pub struct Env {
    memory: Option<Memory>,
    func: Option<Function>,
    world: World,
}

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
            let registry = data.world.get_resource::<AppTypeRegistry>().unwrap();
            let registry_ref = registry.read();
            let type_registration = registry_ref.get_with_type_path(&type_path).unwrap();
            let component_id = data
                .world
                .components()
                .get_resource_id(type_registration.type_id())
                .unwrap();
            let ptr = data.world.get_resource_by_id(component_id).unwrap();
            let value: &ExampleResource = unsafe { ptr.deref() };

            let json = serde_json::to_value(value).unwrap();
            Response::Resource(json)
        }
        Request::SetResource { type_path, value } => {
            let registry = data.world.get_resource::<AppTypeRegistry>().unwrap();
            let registry_ref = registry.read();
            let type_registration = registry_ref.get_with_type_path(&type_path).unwrap();
            let component_id = data
                .world
                .components()
                .get_resource_id(type_registration.type_id())
                .unwrap();
            drop(registry_ref);

            let mut ptr = data.world.get_resource_mut_by_id(component_id).unwrap();
            let target: &mut ExampleResource = unsafe { ptr.as_mut().deref_mut() };
            *target = serde_json::from_value(value).unwrap();

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
