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

pub use wavedash_macros::main;

pub mod prelude {
    pub use crate::{Res, ResMut, Update, World};
}

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
    let mut world = unsafe { World::current_inner() };
    RUNTIME
        .try_with(|rt| rt.borrow_mut().systems.get_mut(&id).unwrap()(&mut world))
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

type SystemFn = Box<dyn FnMut(&mut World)>;

struct Runtime {
    systems: HashMap<i32, SystemFn>,
}

thread_local! {
    static RUNTIME: RefCell<Runtime> = RefCell::new(Runtime {
        systems: HashMap::new(),
    });
}

#[derive(TypePath)]
pub struct Update;

pub struct World {
    _priv: (),
}

impl World {
    pub unsafe fn current() -> Self {
        Self { _priv: () }
    }

    unsafe fn current_inner() -> Self {
        Self { _priv: () }
    }

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

    /// # Safety
    /// TODO
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

pub trait WasmSystemParamFunction<Marker> {
    type Params: WasmSystemParam;

    fn run(&mut self, params: <Self::Params as WasmSystemParam>::Item<'_>);
}

macro_rules! impl_tuples {
    ($($params:tt),*) => {
        impl<$($params: WasmSystemParam),*> WasmSystemParam for ($($params,)*) {
            type Item<'w> = ($($params::Item<'w>,)*);

            unsafe fn from_wasm_world(world: &mut World) -> Self::Item<'_> {
                let ptr = world as *mut _;
                ($($params::from_wasm_world(&mut *ptr),)*)
            }
        }

        impl<F, $($params : WasmSystemParam),*> WasmSystemParamFunction<fn(($($params,)*))> for F
        where
            for<'a> &'a mut F: FnMut($($params),*) + FnMut($($params::Item<'_>),*),
            F: Send + Sync + 'static,

        {
            type Params = ($($params,)*);

            fn run(&mut self, params: <Self::Params as WasmSystemParam>::Item<'_>) {
                #[allow(non_snake_case)]
                fn call<$($params),*>(mut f: impl FnMut($($params),*), ($($params,)*): ($($params,)*)) {
                    f($($params),*);
                }

                call(self, params);
            }
        }
    };
}

impl_tuples!(P1);
impl_tuples!(P1, P2);
impl_tuples!(P1, P2, P3);
impl_tuples!(P1, P2, P3, P4);
impl_tuples!(P1, P2, P3, P4, P5);
impl_tuples!(P1, P2, P3, P4, P5, P6);
impl_tuples!(P1, P2, P3, P4, P5, P6, P7);
impl_tuples!(P1, P2, P3, P4, P5, P6, P7, P8);
