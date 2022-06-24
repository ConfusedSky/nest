#![allow(unsafe_code)]

use std::{ffi::CString, ptr::NonNull};

use wren_sys::{
    self, wrenGetListCount, wrenGetListElement, wrenGetSlotBool, wrenGetSlotBytes,
    wrenGetSlotDouble, wrenGetSlotHandle, wrenGetSlotType, wrenInsertInList, wrenSetSlotBool,
    wrenSetSlotDouble, wrenSetSlotHandle, wrenSetSlotNewList, wrenSetSlotNull, wrenSetSlotString,
};

use super::{Handle, Slot, VMPtr};

struct SlotStorage {
    vm: VMPtr,
    slot: Slot,
    handle: Handle,
}

impl Drop for SlotStorage {
    fn drop(&mut self) {
        unsafe { self.handle.send_to_vm(self.vm, self.slot) }
    }
}

unsafe fn store_slot(vm: VMPtr, slot: Slot) -> SlotStorage {
    let handle = Handle::get_from_vm(vm, slot);
    SlotStorage { vm, slot, handle }
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
    // We are always able to get a handle from wren
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

impl<T: Get> Get for Vec<T> {
    unsafe fn get_from_vm(vm: VMPtr, slot: Slot) -> Self {
        // Store the next slot so we don't overwrite it's value
        // Or use the previous slot instead of juggling slots
        let (_store, item_slot) = if slot == 0 {
            (Some(store_slot(vm, slot + 1)), slot + 1)
        } else {
            (None, slot - 1)
        };

        let mut vec = vec![];

        let count = wrenGetListCount(vm.as_ptr(), slot);

        for i in 0..count {
            wrenGetListElement(vm.as_ptr(), slot, i, item_slot);
            vec.push(T::get_from_vm(vm, item_slot));
        }

        vec
    }
}

impl Value for CString {
    const ADDITIONAL_SLOTS_NEEDED: Slot = 0;
}

impl Set for CString {
    unsafe fn send_to_vm(&self, vm: VMPtr, slot: Slot) {
        wrenSetSlotString(vm.as_ptr(), slot, self.as_ptr());
    }
}

unsafe fn send_string_to_vm<S: AsRef<str>>(vm: VMPtr, value: S, slot: Slot) {
    let str = value.as_ref();
    wren_sys::wrenSetSlotBytes(
        vm.as_ptr(),
        slot,
        str.as_ptr().cast(),
        // The len should always be valid
        str.len().try_into().expect("Invalid length for str"),
    );
}

impl Value for str {
    const ADDITIONAL_SLOTS_NEEDED: Slot = 0;
}

impl Set for str {
    unsafe fn send_to_vm(&self, vm: VMPtr, slot: Slot) {
        send_string_to_vm(vm, self, slot);
    }
}

impl Value for String {
    const ADDITIONAL_SLOTS_NEEDED: Slot = 0;
}

impl Set for String {
    unsafe fn send_to_vm(&self, vm: VMPtr, slot: Slot) {
        send_string_to_vm(vm, self, slot);
    }
}

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
        let t = wrenGetSlotType(vm.as_ptr(), slot);
        match t {
            wren_sys::WrenType_WREN_TYPE_BOOL => wrenGetSlotBool(vm.as_ptr(), slot),
            wren_sys::WrenType_WREN_TYPE_NULL => false,
            _ => true,
        }
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
    use crate::wren::{Handle, VMPtr, Vm, VmUserData};

    use super::SetArgs;

    struct TestUserData;
    impl VmUserData for TestUserData {}

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

    macro_rules! make_args {
        ($class:ident, $($args:tt),+) => {
            &(&$class, $( &$args )+)
        };
        ($class:ident) => {
            &(&$class)
        };
    }

    unsafe fn make_call(vm: VMPtr, method: &Handle, args: &impl SetArgs) -> bool {
        vm.set_stack(args);
        vm.call(method).unwrap();
        vm.get_return_value::<bool>()
    }

    macro_rules! make_call {
        ($class:ident.$handle:ident($vm:ident)) => {{
            make_call($vm, &$handle, (make_args!($class)))
        }};
        ($class:ident.$handle:ident($vm:ident, $($args:expr),+ )) => {{
            make_call($vm, &$handle, (make_args!($class, $($args),+)))
        }};
    }

    fn create_test_vm(source: &str) -> (Vm<TestUserData>, VMPtr, Handle) {
        let vm = Vm::new(TestUserData).expect("VM shouldn't fail to initialize");

        vm.interpret("<test>", source)
            .expect("Code should run successfully");

        let vmptr = vm.get_ptr();

        vmptr.ensure_slots(1);
        let class = unsafe { vmptr.get_variable_unchecked("<test>", "Test", 0) };

        (vm, vmptr, class)
    }

    // Test that all values other than null and false are falsy
    #[allow(non_snake_case)]
    #[test]
    fn test_bool() {
        use crate::wren::make_call_handle;
        let source = "class Test {
                static returnTrue() { true }
                static returnFalse() { false }
                static returnNull() { null }
                static returnValue(value) { value }
            }";

        let (x, vm, Test) = create_test_vm(source);
        let returnTrue = make_call_handle!(vm, "returnTrue()");
        let returnFalse = make_call_handle!(vm, "returnFalse()");
        let returnNull = make_call_handle!(vm, "returnNull()");
        let returnValue = make_call_handle!(vm, "returnValue(_)");

        unsafe {
            // False cases
            assert!(!make_call!(Test.returnNull(vm)));
            assert!(!make_call!(Test.returnFalse(vm)));
            assert!(!make_call!(Test.returnValue(vm, false)));

            // True cases
            assert!(make_call!(Test.returnTrue(vm)));
            assert!(make_call!(Test.returnValue(vm, "".to_string())));
            assert!(make_call!(Test.returnValue(vm, Test)));
            assert!(make_call!(Test.returnValue(vm, vec![1.0])));
            assert!(make_call!(Test.returnValue(vm, 1.0)));
        }

        // Make sure vm lives long enough
        // TODO: Make sure the ptr always outlives the vm
        drop(x);
    }
}
