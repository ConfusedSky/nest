#![allow(unsafe_code)]

use std::{ffi::CString, ptr::NonNull};

use wren_sys::{
    wrenGetSlotBool, wrenGetSlotBytes, wrenGetSlotDouble, wrenGetSlotHandle, wrenInsertInList,
    wrenSetSlotBool, wrenSetSlotDouble, wrenSetSlotHandle, wrenSetSlotNewList, wrenSetSlotNull,
    wrenSetSlotString,
};

use super::{Handle, Slot, VMPtr};

enum WrenValue<'s> {
    Null,
    Bool(bool),
    Number(f64),
    String(&'s str),
    List(Vec<WrenValue<'s>>),
    Handle(Handle),
}

impl<'s> WrenValue<'s> {
    const fn get_required_slots(&self) -> Slot {
        match *self {
            Self::Null | Self::Bool(_) | Self::Number(_) | Self::String(_) | Self::Handle(_) => 0,
            Self::List(_) => 1,
        }
    }
}

/// `WrenValue` is a value that is marshallable from the vm to rust and vice-versa
/// Methods have 3 arguments
/// VM: The vm pointer
/// slot: The slot being saved to
pub trait Value {
    /// Number of additional slots that need to be allocated to use this
    const ADDITIONAL_SLOTS_NEEDED: Slot;
}

pub trait Set: Value {
    unsafe fn send_to_vm(&self, vm: VMPtr, slot: Slot);
}

pub trait Get: Value {
    unsafe fn get_from_vm(vm: VMPtr, slot: Slot) -> Self;
}

// () is implemented to allow skipping slots
// and to set send null to the vm
impl Value for () {
    const ADDITIONAL_SLOTS_NEEDED: Slot = 0;
}

impl Get for () {
    unsafe fn get_from_vm(_vm: VMPtr, _slot: Slot) -> Self {}
}

impl Set for () {
    unsafe fn send_to_vm(&self, vm: VMPtr, slot: Slot) {
        wrenSetSlotNull(vm.as_ptr(), slot);
    }
}

impl<T: Value> Value for &T {
    const ADDITIONAL_SLOTS_NEEDED: Slot = T::ADDITIONAL_SLOTS_NEEDED;
}

impl<T: Set> Set for &T {
    unsafe fn send_to_vm(&self, vm: VMPtr, slot: Slot) {
        (*self).send_to_vm(vm, slot);
    }
}

impl Value for Handle {
    const ADDITIONAL_SLOTS_NEEDED: Slot = 0;
}
impl Set for Handle {
    unsafe fn send_to_vm(&self, vm: VMPtr, slot: Slot) {
        wrenSetSlotHandle(vm.as_ptr(), slot, self.as_ptr());
    }
}
impl Get for Handle {
    unsafe fn get_from_vm(vm: VMPtr, slot: Slot) -> Self {
        let handle = wrenGetSlotHandle(vm.as_ptr(), slot);
        Self::new(vm, NonNull::new_unchecked(handle))
    }
}

unsafe fn send_iterator_to_vm<T: Set, I: Iterator<Item = T>>(iterator: I, vm: VMPtr, slot: Slot) {
    wrenSetSlotNewList(vm.as_ptr(), slot);

    for value in iterator {
        value.send_to_vm(vm, slot + 1);
        wrenInsertInList(vm.as_ptr(), slot, -1, slot + 1);
    }
}

impl<T: Value> Value for Vec<T> {
    // This needs at least one for moving values into the wren list as well as
    // any additional slots for T's initialization
    const ADDITIONAL_SLOTS_NEEDED: Slot = 1 + T::ADDITIONAL_SLOTS_NEEDED;
}

impl<T: Set> Set for Vec<T> {
    unsafe fn send_to_vm(&self, vm: VMPtr, slot: Slot) {
        send_iterator_to_vm(self.iter(), vm, slot);
    }
}

impl<T: Value> Value for [T] {
    // This needs at least one for moving values into the wren list as well as
    // any additional slots for T's initialization
    const ADDITIONAL_SLOTS_NEEDED: Slot = 1 + T::ADDITIONAL_SLOTS_NEEDED;
}

impl<T: Set> Set for [T] {
    unsafe fn send_to_vm(&self, vm: VMPtr, slot: Slot) {
        send_iterator_to_vm(self.iter(), vm, slot);
    }
}

// This probably doesn't work correctly as it's written
// impl<T: Get> Get for Vec<T> {
// unsafe fn get_from_vm(vm: VMPtr, slot: Slot) -> Self {
// let mut vec = vec![];

// let count = wrenGetListCount(vm.as_ptr(), slot);

// for i in 0..count {
// wrenGetListElement(vm.as_ptr(), slot, i, slot + 1);
// vec.push(T::get_from_vm(vm, slot + 1));
// }

// vec
// }
// }

impl Value for CString {
    const ADDITIONAL_SLOTS_NEEDED: Slot = 0;
}

impl Set for CString {
    unsafe fn send_to_vm(&self, vm: VMPtr, slot: Slot) {
        wrenSetSlotString(vm.as_ptr(), slot, self.as_ptr());
    }
}

macro_rules! str_set_impl {
    ($t:ty) => {
        impl Value for $t {
            const ADDITIONAL_SLOTS_NEEDED: Slot = 0;
        }

        impl Set for $t {
            unsafe fn send_to_vm(&self, vm: VMPtr, slot: Slot) {
                wren_sys::wrenSetSlotBytes(
                    vm.as_ptr(),
                    slot,
                    self.as_ptr().cast(),
                    self.len().try_into().unwrap(),
                );
            }
        }
    };
}

str_set_impl!(str);
str_set_impl!(String);

impl Get for String {
    unsafe fn get_from_vm(vm: VMPtr, slot: Slot) -> Self {
        let mut len = 0;
        let ptr = wrenGetSlotBytes(vm.as_ptr(), slot, &mut len).cast();
        let len = len.try_into().unwrap();
        let slice = std::slice::from_raw_parts(ptr, len);
        Self::from_utf8_lossy(slice).to_string()
    }
}

impl Value for f64 {
    const ADDITIONAL_SLOTS_NEEDED: Slot = 0;
}

impl Set for f64 {
    unsafe fn send_to_vm(&self, vm: VMPtr, slot: Slot) {
        wrenSetSlotDouble(vm.as_ptr(), slot, *self);
    }
}

impl Get for f64 {
    unsafe fn get_from_vm(vm: VMPtr, slot: Slot) -> Self {
        wrenGetSlotDouble(vm.as_ptr(), slot)
    }
}

impl Value for bool {
    const ADDITIONAL_SLOTS_NEEDED: Slot = 0;
}

impl Set for bool {
    unsafe fn send_to_vm(&self, vm: VMPtr, slot: Slot) {
        wrenSetSlotBool(vm.as_ptr(), slot, *self);
    }
}

impl Get for bool {
    unsafe fn get_from_vm(vm: VMPtr, slot: Slot) -> Self {
        wrenGetSlotBool(vm.as_ptr(), slot)
    }
}

const fn _const_max_helper(a: Slot, b: Slot) -> Slot {
    [a, b][(a < b) as usize]
}

macro_rules! expand_required_slots {
    (@step $i:expr, $x:ty) => (<$x>::ADDITIONAL_SLOTS_NEEDED+ $i);
    (@step $i:expr, $x:ty, $($y:ty),+ $(,)?) => (
        _const_max_helper(
            <$x>::ADDITIONAL_SLOTS_NEEDED+ $i,
            expand_required_slots!(@step $i + 1, $($y),+),
        )
    );
    ($x:ty, $($y:ty),+ $(,)?) => (
        expand_required_slots!(@step 1i32, $x, $($y),+)
    )
}

macro_rules! expand_set_slots {
    ($self:ident, $vm:ident, $method:ident, $i:tt) => {
        $self.$i.$method($vm, $i);
    };
    ($self:ident, $vm:ident, $method:ident, $i:tt, $($xs:tt),+ $(,)?) => {
        expand_set_slots!($self, $vm, $method, $i);
        expand_set_slots!($self, $vm, $method, $( $xs ), *);
    };
}

macro_rules! impl_set_args {
    ($( $xs:ident = $i:tt ), *) => {
        impl<$( $xs: Set, )*> SetArgs for ($( &$xs, )*) {
            const REQUIRED_SLOTS: Slot =
                expand_required_slots!($( $xs ), *);


            unsafe fn set_slots(&self, vm: VMPtr) {
                expand_set_slots!(self, vm, send_to_vm, $( $i ), *);
            }
        }
    };
}

pub trait SetArgs {
    const REQUIRED_SLOTS: Slot;
    unsafe fn set_slots(&self, vm: VMPtr);
    /// This fn should probably never be used directly since it only existed
    /// before required slots was a constant
    unsafe fn set_wren_stack_unchecked(&self, vm: VMPtr, num_slots: Slot) {
        vm.ensure_slots(num_slots);
        self.set_slots(vm);
    }
    fn set_wren_stack(&self, vm: VMPtr) {
        // This is guarenteed to be safe because we ensured that we had enough
        // slots for T using REQUIRED_SLOTS
        unsafe {
            self.set_wren_stack_unchecked(vm, Self::REQUIRED_SLOTS);
        }
    }
}

impl<T: Set + ?Sized> SetArgs for T {
    const REQUIRED_SLOTS: Slot = 1 + T::ADDITIONAL_SLOTS_NEEDED;
    unsafe fn set_slots(&self, vm: VMPtr) {
        self.send_to_vm(vm, 0);
    }
}

impl_set_args!(T = 0, U = 1);
impl_set_args!(T = 0, U = 1, V = 2);
impl_set_args!(T = 0, U = 1, V = 2, W = 3);
impl_set_args!(T = 0, U = 1, V = 2, W = 3, W2 = 4);
impl_set_args!(T = 0, U = 1, V = 2, W = 3, W2 = 4, W3 = 5);
impl_set_args!(T = 0, U = 1, V = 2, W = 3, W2 = 4, W3 = 5, W4 = 6);

pub trait GetArgs {
    unsafe fn get_slots(vm: VMPtr) -> Self;
}

impl<T: Get> GetArgs for T {
    unsafe fn get_slots(vm: VMPtr) -> Self {
        T::get_from_vm(vm, 0)
    }
}

macro_rules! impl_get_args {
    ($( $xs:ident = $i:tt ), *) => {
        impl<$( $xs: Get, )*> GetArgs for ($( $xs, )*) {
            unsafe fn get_slots(vm: VMPtr) -> Self{
                (
                    $( $xs::get_from_vm(vm, $i) ), *
                )
            }
        }
    };
}

impl_get_args!(T = 0, U = 1);
impl_get_args!(T = 0, U = 1, V = 2);
impl_get_args!(T = 0, U = 1, V = 2, W = 3);
impl_get_args!(T = 0, U = 1, V = 2, W = 3, W2 = 4);
impl_get_args!(T = 0, U = 1, V = 2, W = 3, W2 = 4, W3 = 5);
impl_get_args!(T = 0, U = 1, V = 2, W = 3, W2 = 4, W3 = 5, W4 = 6);

#[cfg(test)]
mod test {
    use super::SetArgs;

    // TODO: Figure out how to test set_wren_stack

    #[test]
    fn test_slot_size() {
        assert_eq!(f64::REQUIRED_SLOTS, 1);
        assert_eq!(<(&f64, &f64)>::REQUIRED_SLOTS, 2);
        assert_eq!(<(&Vec<Vec<f64>>, &f64)>::REQUIRED_SLOTS, 3);
        assert_eq!(<(&f64, &Vec<Vec<f64>>)>::REQUIRED_SLOTS, 4);
        assert_eq!(<(&f64, &f64, &f64)>::REQUIRED_SLOTS, 3);
        assert_eq!(<(&f64, &f64, &f64, &f64)>::REQUIRED_SLOTS, 4);
    }
}
