use std::ptr::NonNull;

use wren_sys::{wrenReleaseHandle, WrenHandle};

use super::VMPtr;

pub struct Handle {
    vm: VMPtr,
    pointer: NonNull<WrenHandle>,
}

impl Handle {
    pub(crate) const fn new(vm: VMPtr, pointer: NonNull<WrenHandle>) -> Self {
        Self { vm, pointer }
    }

    pub(crate) const fn as_ptr(&self) -> *mut WrenHandle {
        self.pointer.as_ptr()
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        unsafe { wrenReleaseHandle(self.vm.0.as_ptr(), self.pointer.as_ptr()) }
    }
}
