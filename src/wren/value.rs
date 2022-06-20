#![allow(unsafe_code)]

use std::ffi::CString;

use wren_sys::{wrenGetListCount, wrenGetListElement, wrenGetSlotString, wrenSetSlotDouble};

use super::{Handle, Slot, VMPtr};

/// `WrenValue` is a value that is marshallable from the vm to rust and vice-versa
/// Methods have 3 arguments
/// VM: The vm pointer
/// slot: The slot being saved to
pub trait Value {
    /// Number of additional slots that need to be allocated to use this
    const ADDITIONAL_SLOTS_NEEDED: Slot;
    unsafe fn send_to_vm(&self, vm: VMPtr, slot: Slot);
    unsafe fn get_from_vm(vm: VMPtr, slot: Slot) -> Self;
}

impl Value for Handle {
    const ADDITIONAL_SLOTS_NEEDED: Slot = 0;
    unsafe fn send_to_vm(&self, vm: VMPtr, slot: Slot) {
        vm.set_slot_handle_unchecked(slot, *self);
    }
    unsafe fn get_from_vm(vm: VMPtr, slot: Slot) -> Self {
        vm.get_slot_handle_unchecked(slot)
    }
}

impl<T: Value> Value for Vec<T> {
    // This needs at least one for moving values into the wren list as well as
    // any additional slots for T's initialization
    const ADDITIONAL_SLOTS_NEEDED: Slot = 1 + T::ADDITIONAL_SLOTS_NEEDED;
    unsafe fn send_to_vm(&self, vm: VMPtr, slot: Slot) {
        vm.set_slot_new_list_unchecked(slot);

        for value in self {
            value.send_to_vm(vm, slot + 1);
            vm.insert_in_list(slot, -1, slot + 1);
        }
    }
    unsafe fn get_from_vm(vm: VMPtr, slot: Slot) -> Self {
        let mut vec = vec![];

        let count = wrenGetListCount(vm.0.as_ptr(), slot);

        for i in 0..count {
            wrenGetListElement(vm.0.as_ptr(), slot, i, slot + 1);
            vec.push(T::get_from_vm(vm, slot + 1));
        }

        vec
    }
}

impl Value for String {
    const ADDITIONAL_SLOTS_NEEDED: Slot = 0;
    unsafe fn send_to_vm(&self, vm: VMPtr, slot: Slot) {
        vm.set_slot_string_unchecked(slot, self);
    }
    unsafe fn get_from_vm(vm: VMPtr, slot: Slot) -> Self {
        let res = wrenGetSlotString(vm.0.as_ptr(), slot);
        let res = CString::from_raw(res as *mut i8);
        res.to_string_lossy().to_string()
    }
}

impl Value for f64 {
    const ADDITIONAL_SLOTS_NEEDED: Slot = 0;
    unsafe fn send_to_vm(&self, vm: VMPtr, slot: Slot) {
        wrenSetSlotDouble(vm.0.as_ptr(), slot, *self);
    }
    unsafe fn get_from_vm(vm: VMPtr, slot: Slot) -> Self {
        vm.get_slot_double_unchecked(slot)
    }
}

impl Value for bool {
    const ADDITIONAL_SLOTS_NEEDED: Slot = 0;
    unsafe fn send_to_vm(&self, vm: VMPtr, slot: Slot) {
        vm.set_slot_bool_unchecked(slot, *self);
    }
    unsafe fn get_from_vm(vm: VMPtr, slot: Slot) -> Self {
        vm.get_slot_bool_unchecked(slot)
    }
}

pub trait Args {
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
        // slots for T using get_required_slots
        unsafe {
            self.set_wren_stack_unchecked(vm, Self::REQUIRED_SLOTS);
        }
    }
}

impl<T: Value> Args for T {
    const REQUIRED_SLOTS: Slot = 1 + T::ADDITIONAL_SLOTS_NEEDED;
    unsafe fn set_slots(&self, vm: VMPtr) {
        self.send_to_vm(vm, 0);
    }
}

// TODO: Convert this implementation to a macro
// impl<T: Value, U: Value> Args for (&T, &U) {
// const REQUIRED_SLOTS: Slot = const_max!(
// T::ADDITIONAL_SLOTS_NEEDED + 1,
// U::ADDITIONAL_SLOTS_NEEDED + 2,
// );

// unsafe fn set_slots(&self, vm: VMPtr) {
// self.0.send_to_vm(vm, 0);
// self.1.send_to_vm(vm, 1);
// }
// }

// impl<T: Value, U: Value, V: Value> Args for (&T, &U, &V) {
// const REQUIRED_SLOTS: Slot = const_max!(
// T::ADDITIONAL_SLOTS_NEEDED + 1,
// U::ADDITIONAL_SLOTS_NEEDED + 2,
// V::ADDITIONAL_SLOTS_NEEDED + 3,
// );

// unsafe fn set_slots(&self, vm: VMPtr) {
// self.0.send_to_vm(vm, 0);
// self.1.send_to_vm(vm, 1);
// self.2.send_to_vm(vm, 2);
// }
// }

// impl<T: Value, U: Value, V: Value, W: Value> Args for (&T, &U, &V, &W) {
// const REQUIRED_SLOTS: Slot = const_max!(
// T::ADDITIONAL_SLOTS_NEEDED + 1,
// U::ADDITIONAL_SLOTS_NEEDED + 2,
// V::ADDITIONAL_SLOTS_NEEDED + 3,
// W::ADDITIONAL_SLOTS_NEEDED + 4,
// );

// unsafe fn set_slots(&self, vm: VMPtr) {
// self.0.send_to_vm(vm, 0);
// self.1.send_to_vm(vm, 1);
// self.2.send_to_vm(vm, 2);
// self.3.send_to_vm(vm, 3);
// }
// }

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

macro_rules! impl_args {
    ($( $xs:ident ), *) => {
        impl<$( $xs: Value, )*> Args for ($( &$xs, )*) {
            const REQUIRED_SLOTS: Slot =
                expand_required_slots!($( $xs ), *);


            unsafe fn set_slots(&self, vm: VMPtr) {
                self.0.send_to_vm(vm, 0);
                self.1.send_to_vm(vm, 1);
            }
        }
    };
}

impl_args!(T, U);
impl_args!(T, U, V);
impl_args!(T, U, V, W);
impl_args!(T, U, V, W, W2);
impl_args!(T, U, V, W, W2, W3);
impl_args!(T, U, V, W, W2, W3, W4);

#[cfg(test)]
mod test {
    use super::Args;

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
