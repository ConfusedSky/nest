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
    fn get_required_slots(&self) -> Slot;
    unsafe fn set_slots(&self, vm: VMPtr);
    unsafe fn set_wren_stack_unchecked(&self, vm: VMPtr, num_slots: Slot) {
        vm.ensure_slots(num_slots);
        self.set_slots(vm);
    }
    fn set_wren_stack(&self, vm: VMPtr) {
        // This is guarenteed to be safe because we ensured that we had enough
        // slots for T using get_required_slots
        unsafe {
            self.set_wren_stack_unchecked(vm, self.get_required_slots());
        }
    }
}

impl<T: Value> Args for T {
    fn get_required_slots(&self) -> Slot {
        1 + T::ADDITIONAL_SLOTS_NEEDED
    }
    unsafe fn set_slots(&self, vm: VMPtr) {
        self.send_to_vm(vm, 0);
    }
}

// TODO: Convert this implementation to a macro
impl<T: Value, U: Value> Args for (&T, &U) {
    fn get_required_slots(&self) -> Slot {
        [
            T::ADDITIONAL_SLOTS_NEEDED + 1,
            U::ADDITIONAL_SLOTS_NEEDED + 2,
        ]
        .into_iter()
        .max()
        .unwrap_or(1)
    }

    unsafe fn set_slots(&self, vm: VMPtr) {
        self.0.send_to_vm(vm, 0);
        self.1.send_to_vm(vm, 1);
    }
}

impl<T: Value, U: Value, V: Value> Args for (&T, &U, &V) {
    fn get_required_slots(&self) -> Slot {
        [
            T::ADDITIONAL_SLOTS_NEEDED + 1,
            U::ADDITIONAL_SLOTS_NEEDED + 2,
            V::ADDITIONAL_SLOTS_NEEDED + 3,
        ]
        .into_iter()
        .max()
        .unwrap_or(1)
    }

    unsafe fn set_slots(&self, vm: VMPtr) {
        self.0.send_to_vm(vm, 0);
        self.1.send_to_vm(vm, 1);
        self.2.send_to_vm(vm, 2);
    }
}

impl<T: Value, U: Value, V: Value, W: Value> Args for (&T, &U, &V, &W) {
    fn get_required_slots(&self) -> Slot {
        [
            T::ADDITIONAL_SLOTS_NEEDED + 1,
            U::ADDITIONAL_SLOTS_NEEDED + 2,
            V::ADDITIONAL_SLOTS_NEEDED + 3,
            W::ADDITIONAL_SLOTS_NEEDED + 4,
        ]
        .into_iter()
        .max()
        .unwrap_or(1)
    }

    unsafe fn set_slots(&self, vm: VMPtr) {
        self.0.send_to_vm(vm, 0);
        self.1.send_to_vm(vm, 1);
        self.2.send_to_vm(vm, 2);
        self.3.send_to_vm(vm, 3);
    }
}

#[cfg(test)]
mod test {
    use super::Args;

    // TODO: Figure out how to test set_wren_stack

    #[test]
    fn test_slot_size() {
        assert_eq!(1.0.get_required_slots(), 1);
        assert_eq!((&1.0, &2.0).get_required_slots(), 2);
        assert_eq!((&vec![vec![1.0]], &2.0).get_required_slots(), 3);
        assert_eq!((&2.0, &vec![vec![1.0]]).get_required_slots(), 4);
        assert_eq!((&1.0, &2.0, &3.0).get_required_slots(), 3);
        assert_eq!((&1.0, &2.0, &3.0, &4.0).get_required_slots(), 4);
    }
}
