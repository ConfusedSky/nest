#![allow(unsafe_code)]

use std::ffi::CString;

use wren_sys::{wrenGetSlotString, wrenSetSlotDouble};

use super::{Handle, Slot, VMPtr};

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
        vm.set_slot_handle_unchecked(slot, *self);
    }
}
impl Get for Handle {
    unsafe fn get_from_vm(vm: VMPtr, slot: Slot) -> Self {
        vm.get_slot_handle_unchecked(slot)
    }
}

unsafe fn send_iterator_to_vm<T: Set, I: Iterator<Item = T>>(iterator: I, vm: VMPtr, slot: Slot) {
    vm.set_slot_new_list_unchecked(slot);

    for value in iterator {
        value.send_to_vm(vm, slot + 1);
        vm.insert_in_list(slot, -1, slot + 1);
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

// let count = wrenGetListCount(vm.0.as_ptr(), slot);

// for i in 0..count {
// wrenGetListElement(vm.0.as_ptr(), slot, i, slot + 1);
// vec.push(T::get_from_vm(vm, slot + 1));
// }

// vec
// }
// }

macro_rules! str_set_impl {
    ($t:ty) => {
        impl Value for $t {
            const ADDITIONAL_SLOTS_NEEDED: Slot = 0;
        }

        impl Set for $t {
            unsafe fn send_to_vm(&self, vm: VMPtr, slot: Slot) {
                vm.set_slot_string_unchecked(slot, self);
            }
        }
    };
}

str_set_impl!(str);
str_set_impl!(String);

impl Get for String {
    unsafe fn get_from_vm(vm: VMPtr, slot: Slot) -> Self {
        let res = wrenGetSlotString(vm.0.as_ptr(), slot);
        let res = CString::from_raw(res as *mut i8);
        res.to_string_lossy().to_string()
    }
}

impl Value for f64 {
    const ADDITIONAL_SLOTS_NEEDED: Slot = 0;
}

impl Set for f64 {
    unsafe fn send_to_vm(&self, vm: VMPtr, slot: Slot) {
        wrenSetSlotDouble(vm.0.as_ptr(), slot, *self);
    }
}

impl Get for f64 {
    unsafe fn get_from_vm(vm: VMPtr, slot: Slot) -> Self {
        vm.get_slot_double_unchecked(slot)
    }
}

impl Value for bool {
    const ADDITIONAL_SLOTS_NEEDED: Slot = 0;
}

impl Set for bool {
    unsafe fn send_to_vm(&self, vm: VMPtr, slot: Slot) {
        vm.set_slot_bool_unchecked(slot, *self);
    }
}

impl Get for bool {
    unsafe fn get_from_vm(vm: VMPtr, slot: Slot) -> Self {
        vm.get_slot_bool_unchecked(slot)
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

impl<T: Set> SetArgs for T {
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
