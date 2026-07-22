//! This file contains safe memory types

#![allow(dead_code)]

pub(self) mod boxed;
pub(self) mod rc;
pub(self) mod string;
pub(self) mod vec;
pub use boxed::Box;
pub use rc::Rc;
pub use string::String;
pub use vec::Vec;

pub(self) mod prelude {
    pub use super::super::alloc::{alloc, alloc_t, dealloc, dealloc_t, realloc, realloc_t};

    pub use core::{
        alloc::Layout,
        ops::{Deref, DerefMut},
        ptr::{null, null_mut},
    };
}
