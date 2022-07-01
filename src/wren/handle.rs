#![allow(clippy::module_name_repetitions)]

use std::{ffi::CStr, marker::PhantomData, ops::Deref, ptr::NonNull};

use wren_sys::{self as ffi, wrenReleaseHandle, WrenHandle};

use super::RawForeignContext;

pub struct Handle<'wren> {
    vm: RawForeignContext<'wren>,
    pointer: NonNull<WrenHandle>,
    phantom: PhantomData<WrenHandle>,
}

impl<'wren> Handle<'wren> {
    pub(crate) fn new(vm: &RawForeignContext<'wren>, pointer: NonNull<WrenHandle>) -> Self {
        Self {
            vm: vm.clone(),
            pointer,
            phantom: PhantomData,
        }
    }

    pub(crate) const fn as_ptr(&self) -> *mut WrenHandle {
        self.pointer.as_ptr()
    }
}

impl<'wren> Drop for Handle<'wren> {
    fn drop(&mut self) {
        unsafe { wrenReleaseHandle(self.vm.as_ptr(), self.pointer.as_ptr()) }
    }
}

/// This is just a thin wrapper around a handle so we can guarente it's a call handle
pub struct CallHandle<'wren>(Handle<'wren>);

impl<'wren> Deref for CallHandle<'wren> {
    type Target = Handle<'wren>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub fn make_call_handle<'wren>(
    vm: &mut RawForeignContext<'wren>,
    signature: &CStr,
) -> CallHandle<'wren> {
    unsafe {
        // SAFETY: this function is always safe to call but may be unsafe to use the handle it returns
        // as that handle might not be valid and safe to use
        let ptr = ffi::wrenMakeCallHandle(vm.as_ptr(), signature.as_ptr());
        CallHandle(Handle::new(vm, NonNull::new_unchecked(ptr)))
    }
}
