use bevy_ecs::system::Resource;
use bevy_reflect::TypePath;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::fmt;
use std::ops::{Deref, DerefMut};
use wavedash_core::{Request, Response};

#[link(wasm_import_module = "__wavedash__")]
extern "C" {
    fn __wavedash_log(ptr: i32, len: i32) -> i32;

    fn __wavedash_request(ptr: i32, len: i32) -> i32;
}

thread_local! {
    static STORAGE: RefCell<HashMap<i32, Vec<u8>>> = RefCell::new(HashMap::new());
}

#[no_mangle]
extern "C" fn __wavedash_alloc(len: i32) -> i32 {
    STORAGE
        .try_with(|storage| {
            let mut storage = storage.borrow_mut();
            let buf = vec![0; len as usize];
            let ptr = buf.as_ptr() as _;
            storage.insert(ptr, buf);
            ptr
        })
        .unwrap()
}

#[no_mangle]
extern "C" fn __wavedash_run_system(id: i32) {
    let mut app = unsafe { App::current() };
    RUNTIME
        .try_with(|rt| rt.borrow_mut().systems.get_mut(&id).unwrap()(&mut app.world))
        .unwrap();
}

fn request(req: &Request) -> Response {
    let s = CString::new(serde_json::to_string(req).unwrap()).unwrap();
    let ptr = unsafe { __wavedash_request(s.as_ptr() as _, s.as_bytes().len() as i32) };

    let s = unsafe { CStr::from_ptr(ptr as _) }.to_str().unwrap();
    let response = serde_json::from_str(s).unwrap();

    STORAGE
        .try_with(|storage| {
            storage.borrow_mut().remove(&ptr).unwrap();
        })
        .unwrap();

    response
}

pub fn dbg(s: impl fmt::Debug) {
    request(&Request::Log(format!("{:?}", s)));
}

pub fn log(s: impl fmt::Display) {
    request(&Request::Log(s.to_string()));
}

struct Runtime {
    systems: HashMap<i32, Box<dyn FnMut(&mut World)>>,
    next_id: i32,
}

thread_local! {
    static RUNTIME: RefCell<Runtime> = RefCell::new(Runtime {
        systems: HashMap::new(),
        next_id: 0,
    });
}

pub struct App {
    world: World,
}

impl App {
    pub unsafe fn current() -> Self {
        App {
            world: World { _priv: () },
        }
    }

    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    pub fn add_system<Marker, F>(&mut self, label: impl TypePath, mut system: F)
    where
        F: WasmSystemParamFunction<Marker> + 'static,
    {
        RUNTIME
            .try_with(|rt| {
                let mut rt = rt.borrow_mut();

                let id = rt.next_id;
                rt.next_id += 1;

                rt.systems.insert(
                    id,
                    Box::new(move |world| {
                        let param = unsafe { F::Params::from_wasm_world(world) };
                        system.run(param);
                    }),
                );
            })
            .unwrap();
    }
}

#[derive(TypePath)]
pub struct Update;

pub struct World {
    _priv: (),
}

impl World {
    pub fn resource<R>(&self) -> R
    where
        R: Resource + TypePath + DeserializeOwned,
    {
        let type_path = R::type_path().to_string();
        let res = request(&Request::GetResource { type_path });
        match res {
            Response::Resource(json) => serde_json::from_value(json).unwrap(),
            _ => unimplemented!(),
        }
    }

    pub fn resource_mut<R>(&mut self) -> ResourceMut<R>
    where
        R: Resource + DeserializeOwned + Serialize + TypePath,
    {
        let type_path = R::type_path().to_string();
        let res = request(&Request::GetResource {
            type_path: type_path.clone(),
        });

        match res {
            Response::Resource(json) => ResourceMut {
                resource: serde_json::from_value(json).unwrap(),
                type_path,
                _world: self,
            },
            _ => unimplemented!(),
        }
    }
}

pub struct ResourceMut<'a, R: Serialize> {
    resource: R,
    type_path: String,
    _world: &'a mut World,
}

impl<R: Serialize> Deref for ResourceMut<'_, R> {
    type Target = R;

    fn deref(&self) -> &Self::Target {
        &self.resource
    }
}

impl<R: Serialize> DerefMut for ResourceMut<'_, R> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.resource
    }
}

impl<R: Serialize> Drop for ResourceMut<'_, R> {
    fn drop(&mut self) {
        request(&Request::SetResource {
            type_path: self.type_path.clone(),
            value: serde_json::to_value(&self.resource).unwrap(),
        });
    }
}

pub trait WasmSystemParam {
    type Item<'w>;

    unsafe fn from_wasm_world(world: &mut World) -> Self::Item<'_>;
}

pub struct Res<R> {
    pub resource: R,
}

impl<R> WasmSystemParam for Res<R>
where
    R: Resource + TypePath + DeserializeOwned,
{
    type Item<'w> = Res<R>;

    unsafe fn from_wasm_world(world: &mut World) -> Self::Item<'_> {
        Res {
            resource: world.resource(),
        }
    }
}

pub struct ResMut<'w, R: Serialize> {
    resource: ResourceMut<'w, R>,
}

impl<R: Serialize> Deref for ResMut<'_, R> {
    type Target = R;

    fn deref(&self) -> &Self::Target {
        &self.resource
    }
}

impl<R: Serialize> DerefMut for ResMut<'_, R> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.resource
    }
}

impl<R> WasmSystemParam for ResMut<'_, R>
where
    R: Resource + TypePath + DeserializeOwned + Serialize,
{
    type Item<'w> = ResMut<'w, R>;

    unsafe fn from_wasm_world(world: &mut World) -> Self::Item<'_> {
        ResMut {
            resource: world.resource_mut(),
        }
    }
}

impl<T: WasmSystemParam> WasmSystemParam for (T,) {
    type Item<'w> = T::Item<'w>;

    unsafe fn from_wasm_world(world: &mut World) -> Self::Item<'_> {
        T::from_wasm_world(world)
    }
}

pub trait WasmSystemParamFunction<Marker> {
    type Params: WasmSystemParam;

    fn run(&mut self, params: <Self::Params as WasmSystemParam>::Item<'_>);
}

impl<F, P> WasmSystemParamFunction<P> for F
where
    for<'a> &'a mut F: FnMut(P) + FnMut(P::Item<'_>),
    F: Send + Sync + 'static,
    P: WasmSystemParam,
{
    type Params = P;

    fn run(&mut self, params: <Self::Params as WasmSystemParam>::Item<'_>) {
        fn call<P>(mut f: impl FnMut(P), params: P) {
            f(params);
        }

        call(self, params);
    }
}
