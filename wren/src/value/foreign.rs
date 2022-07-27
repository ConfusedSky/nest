use std::{
    any::Any,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    ptr::NonNull,
};

use crate::{
    context::Location,
    value::{RawContext, TryGetError, TryGetResult},
    GetValue, Handle, Slot, WrenType,
};
use enumflags2::{make_bitflags, BitFlags};
use wren_sys as ffi;

#[derive(Clone, Copy)]
pub struct Foreign<'wren, T: Any> {
    data: NonNull<T>,
    // We don't actually own this data, the VM does
    // it's also not guarenteed to be alive past the
    // foreign method that it is called on
    phantom: PhantomData<&'wren mut T>,
}

impl<'wren, T: Any> Foreign<'wren, T> {
    pub(super) unsafe fn new(data: NonNull<T>) -> Self {
        Self {
            data,
            phantom: PhantomData::default(),
        }
    }

    /// # Safety
    /// must be called on a valid slot that is already known to
    /// contain a foreign object
    ///
    /// # Errors
    /// If the foreign contained in the slot is of a wrong type spring an error
    pub unsafe fn try_get_slot_unchecked<L: Location>(
        vm: &mut RawContext<'wren, L>,
        slot: Slot,
    ) -> TryGetResult<'wren, Self> {
        let foreign: *mut Box<dyn Any> = ffi::wrenGetSlotForeign(vm.as_ptr(), slot).cast();

        // There are two levels of indirection here since we need to
        // follow the pointer then follow the box
        let foreign_any = &mut **foreign as &mut dyn Any;
        let data = foreign_any.downcast_mut::<T>();

        if let Some(data) = data {
            let data = NonNull::new_unchecked(data);
            Ok(Self::new(data))
        } else {
            Err(TryGetError::IncompatibleForeign)
        }
    }
}

impl<'wren, T: Any> AsRef<T> for Foreign<'wren, T> {
    fn as_ref(&self) -> &T {
        unsafe { &*self.data.as_ptr() }
    }
}

impl<'wren, T: Any> AsMut<T> for Foreign<'wren, T> {
    fn as_mut(&mut self) -> &mut T {
        unsafe { &mut *self.data.as_ptr() }
    }
}

impl<'wren, T: Any> Deref for Foreign<'wren, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<'wren, T: Any> DerefMut for Foreign<'wren, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut()
    }
}

impl<'wren, T: Any, L: Location> GetValue<'wren, L> for Foreign<'wren, T> {
    const COMPATIBLE_TYPES: BitFlags<WrenType> = make_bitflags!(WrenType::{Foreign});

    unsafe fn get_slot_unchecked(
        vm: &mut RawContext<'wren, L>,
        slot: Slot,
        slot_type: WrenType,
    ) -> Self {
        assert!(slot_type == WrenType::Foreign);
        Self::try_get_slot_unchecked(vm, slot).unwrap()
    }

    unsafe fn try_get_slot(
        vm: &mut RawContext<'wren, L>,
        slot: Slot,
        get_handle: bool,
    ) -> TryGetResult<'wren, Self>
    where
        Self: Sized,
    {
        let slot_type = vm.get_slot_type(slot);
        if <Self as GetValue<'wren, L>>::COMPATIBLE_TYPES.contains(slot_type) {
            Self::try_get_slot_unchecked(vm, slot)
        } else {
            Err(TryGetError::IncompatibleType(if get_handle {
                Some(Handle::get_slot_unchecked(vm, slot, slot_type))
            } else {
                None
            }))
        }
    }
}
