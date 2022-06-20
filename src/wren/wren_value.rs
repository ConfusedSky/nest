#![allow(unsafe_code)]

use std::ffi::CString;

use wren_sys::{wrenGetListCount, wrenGetListElement, wrenGetSlotString, wrenSetSlotDouble};

use super::{Handle, Slot, VMPtr};

/// `WrenValue` is a value that is marshallable from the vm to rust and vice-versa
/// Methods have 3 arguments
/// VM: The vm pointer
/// slot: The slot being saved to
pub trait WrenValue {
    /// Number of additional slots that need to be allocated to use this
    const ADDITIONAL_SLOTS_NEEDED: Slot;
    unsafe fn send_to_vm(&self, vm: VMPtr, slot: Slot);
    unsafe fn get_from_vm(vm: VMPtr, slot: Slot) -> Self;
}

impl WrenValue for Handle {
    const ADDITIONAL_SLOTS_NEEDED: Slot = 0;
    unsafe fn send_to_vm(&self, vm: VMPtr, slot: Slot) {
        vm.set_slot_handle_unchecked(slot, *self);
    }
    unsafe fn get_from_vm(vm: VMPtr, slot: Slot) -> Self {
        vm.get_slot_handle_unchecked(slot)
    }
}

impl<T: WrenValue> WrenValue for Vec<T> {
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

impl WrenValue for String {
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

impl WrenValue for f64 {
    const ADDITIONAL_SLOTS_NEEDED: Slot = 0;
    unsafe fn send_to_vm(&self, vm: VMPtr, slot: Slot) {
        wrenSetSlotDouble(vm.0.as_ptr(), slot, *self);
    }
    unsafe fn get_from_vm(vm: VMPtr, slot: Slot) -> Self {
        vm.get_slot_double_unchecked(slot)
    }
}

impl WrenValue for bool {
    const ADDITIONAL_SLOTS_NEEDED: Slot = 0;
    unsafe fn send_to_vm(&self, vm: VMPtr, slot: Slot) {
        vm.set_slot_bool_unchecked(slot, *self)
    }
    unsafe fn get_from_vm(vm: VMPtr, slot: Slot) -> Self {
        vm.get_slot_bool_unchecked(slot)
    }
}
