use bevy_ecs::prelude::*;
use bevy_reflect::TypePath;
use serde::de::DeserializeOwned;
use std::{alloc, ffi::CString, mem, slice, str::FromStr};
use wavedash_core::Buffer;
pub use wavedash_macros::main;

pub mod prelude {}

#[link(wasm_import_module = "__wavedash__")]
unsafe extern "C" {
    fn __wavedash_get_resource(world_ptr: i64, type_path_ptr: i32, type_path_len: i32) -> Buffer;
}

#[unsafe(no_mangle)]
extern "C" fn __wavedash_alloc(size: i32, align: i32) -> i32 {
    let layout = alloc::Layout::from_size_align(size as usize, align as usize).unwrap();
    let ptr = unsafe { alloc::alloc(layout) };
    ptr as i32
}

pub struct World {
    ptr: i64,
}

impl World {
    pub unsafe fn new(ptr: i64) -> Self {
        Self { ptr }
    }

    pub fn resource<R>(&self) -> R
    where
        R: Resource + TypePath + DeserializeOwned,
    {
        let type_path = CString::from_str(&R::type_path().to_string()).unwrap();
        let type_path_bytes = type_path.as_bytes_with_nul();
        let buf = unsafe {
            __wavedash_get_resource(
                self.ptr,
                type_path_bytes.as_ptr() as _,
                type_path_bytes.len() as i32,
            )
        };

        let bytes = unsafe { slice::from_raw_parts(buf.ptr as *const u8, buf.len as usize) };
        let r = bincode::deserialize(bytes).unwrap();

        unsafe {
            alloc::dealloc(
                buf.ptr as *mut u8,
                alloc::Layout::from_size_align(buf.len as usize, mem::align_of::<u8>()).unwrap(),
            );
        }

        r
    }
}
