#![allow(clippy::module_name_repetitions)]

use std::{
    ffi::{CStr, FromBytesWithNulError},
    fmt::Debug,
    marker::PhantomData,
    ops::Deref,
    ptr::NonNull,
};

use wren_sys::{self as ffi, wrenReleaseHandle, WrenHandle};

use super::{
    context::{Context, Location},
    RawForeignContext,
};

pub struct Handle<'wren> {
    vm: RawForeignContext<'wren>,
    pointer: NonNull<WrenHandle>,
    phantom: PhantomData<WrenHandle>,
}

impl<'wren> Debug for Handle<'wren> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Handle").field(&self.as_ptr()).finish()
    }
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
pub struct CallHandle<'wren> {
    handle: Handle<'wren>,
    argument_count: usize,
}

impl<'wren> Deref for CallHandle<'wren> {
    type Target = Handle<'wren>;
    fn deref(&self) -> &Self::Target {
        &self.handle
    }
}

impl<'wren> CallHandle<'wren> {
    /// NOTE this could probably become a constant with
    /// some macro trickery, but for now this is fine
    pub const fn get_argument_count(&self) -> usize {
        self.argument_count
    }

    // TODO: make most of our handles with a macro
    // that creates a signature with call_signature!
    // and this function. Since that has no runtime cost
    pub unsafe fn new_unchecked<L: Location, V>(
        vm: &mut Context<'wren, V, L>,
        signature: &CStr,
        argument_count: usize,
    ) -> Self {
        // SAFETY: this function is always safe to call but may be unsafe to use the handle it returns
        // as that handle might not be valid and safe to use
        let ptr = ffi::wrenMakeCallHandle(vm.as_ptr(), signature.as_ptr());
        let handle = Handle::new(
            vm.as_foreign_mut().as_raw_mut(),
            NonNull::new_unchecked(ptr),
        );
        CallHandle {
            handle,
            argument_count,
        }
    }

    pub fn new_from_signature<L: Location, V>(
        vm: &mut Context<'wren, V, L>,
        signature: &CStr,
    ) -> Self {
        let mut argument_count = 0;

        signature
            .to_bytes()
            .iter()
            .skip_while(|byte| **byte != b'(')
            .filter(|byte| **byte == b'_')
            .for_each(|_| argument_count += 1);

        unsafe { Self::new_unchecked(vm, signature, argument_count) }
    }

    pub fn new_from_slice<L: Location, V>(
        vm: &mut Context<'wren, V, L>,
        signature: &[u8],
    ) -> std::result::Result<Self, FromBytesWithNulError> {
        let cstr = CStr::from_bytes_with_nul(signature)?;
        Ok(Self::new_from_signature(vm, cstr))
    }
}

#[cfg(test)]
mod test {
    use crate::wren::test::create_test_vm;

    use super::CallHandle;
    use wren_macros::call_signature;

    #[test]
    fn test_new_from_slice() {
        let source = "class Test {
        }";

        let (mut vm, _) = create_test_vm(source, |_| {});
        let context = vm.get_context();

        assert!(
            CallHandle::new_from_slice(context, call_signature!(Test))
                .unwrap()
                .argument_count
                == 0
        );
        assert!(
            CallHandle::new_from_slice(context, call_signature!(Test, 0))
                .unwrap()
                .argument_count
                == 0
        );
        assert!(
            CallHandle::new_from_slice(context, call_signature!(Test, 1))
                .unwrap()
                .argument_count
                == 1
        );
        assert!(
            CallHandle::new_from_slice(context, call_signature!(Test, 2))
                .unwrap()
                .argument_count
                == 2
        );
        assert!(
            CallHandle::new_from_slice(context, call_signature!(Test, 3))
                .unwrap()
                .argument_count
                == 3
        );
    }
}
