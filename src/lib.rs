#![cfg_attr(feature = "simd", feature(portable_simd))]
#![allow(const_item_mutation)]
#![allow(unused_imports)]

pub use sparrow_core::*;
pub use sparrow_core;

// --- fork additions (NOT in upstream): C# / JNI bindings ---
pub mod ffi;
pub mod ffi_step;
#[cfg(feature = "jni")]
pub mod jni_glue;

