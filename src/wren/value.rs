#![allow(unsafe_code)]

use std::{ffi::CString, ptr::NonNull};

use wren_sys as ffi;

use super::{Handle, RawVMContext, Slot};

struct SlotStorage<'wren> {
    vm: RawVMContext<'wren>,
    slot: Slot,
    handle: Handle<'wren>,
}

impl<'wren> Drop for SlotStorage<'wren> {
    fn drop(&mut self) {
        unsafe { self.handle.send_to_vm(&mut self.vm, self.slot) }
    }
}

unsafe fn store_slot<'wren>(vm: &mut RawVMContext<'wren>, slot: Slot) -> SlotStorage<'wren> {
    let handle = Handle::get_from_vm(vm, slot);
    SlotStorage {
        vm: vm.clone(),
        slot,
        handle,
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

pub trait Set<'wren>: Value {
    unsafe fn send_to_vm(&self, vm: &mut RawVMContext<'wren>, slot: Slot);
}

pub trait Get<'wren>: Value {
    unsafe fn get_from_vm(vm: &mut RawVMContext<'wren>, slot: Slot) -> Self;
}

// () is implemented to allow skipping slots
// and to set send null to the vm
impl Value for () {
    const ADDITIONAL_SLOTS_NEEDED: Slot = 0;
}

impl<'wren> Get<'wren> for () {
    unsafe fn get_from_vm(_vm: &mut RawVMContext<'wren>, _slot: Slot) -> Self {}
}

impl<'wren> Set<'wren> for () {
    unsafe fn send_to_vm(&self, vm: &mut RawVMContext<'wren>, slot: Slot) {
        ffi::wrenSetSlotNull(vm.as_ptr(), slot);
    }
}

impl<T: Value> Value for &T {
    const ADDITIONAL_SLOTS_NEEDED: Slot = T::ADDITIONAL_SLOTS_NEEDED;
}

impl<'wren, T: Set<'wren>> Set<'wren> for &T {
    unsafe fn send_to_vm(&self, vm: &mut RawVMContext<'wren>, slot: Slot) {
        (*self).send_to_vm(vm, slot);
    }
}

impl<'wren> Value for Handle<'wren> {
    const ADDITIONAL_SLOTS_NEEDED: Slot = 0;
}
impl<'wren> Set<'wren> for Handle<'wren> {
    unsafe fn send_to_vm(&self, vm: &mut RawVMContext<'wren>, slot: Slot) {
        ffi::wrenSetSlotHandle(vm.as_ptr(), slot, self.as_ptr());
    }
}
impl<'wren> Get<'wren> for Handle<'wren> {
    // We are always able to Get<'wren> a handle from wren
    unsafe fn get_from_vm(vm: &mut RawVMContext<'wren>, slot: Slot) -> Self {
        let handle = ffi::wrenGetSlotHandle(vm.as_ptr(), slot);
        Self::new(vm, NonNull::new_unchecked(handle))
    }
}

unsafe fn send_iterator_to_vm<'wren, T: Set<'wren>, I: Iterator<Item = T>>(
    iterator: I,
    vm: &mut RawVMContext<'wren>,
    slot: Slot,
) {
    ffi::wrenSetSlotNewList(vm.as_ptr(), slot);

    for value in iterator {
        value.send_to_vm(vm, slot + 1);
        ffi::wrenInsertInList(vm.as_ptr(), slot, -1, slot + 1);
    }
}

impl<T: Value> Value for Vec<T> {
    // This needs at least one for moving values into the wren list as well as
    // any additional slots for T's initialization
    const ADDITIONAL_SLOTS_NEEDED: Slot = 1 + T::ADDITIONAL_SLOTS_NEEDED;
}

impl<'wren, T: Set<'wren>> Set<'wren> for Vec<T> {
    unsafe fn send_to_vm(&self, vm: &mut RawVMContext<'wren>, slot: Slot) {
        send_iterator_to_vm(self.iter(), vm, slot);
    }
}

impl<T: Value> Value for [T] {
    // This needs at least one for moving values into the wren list as well as
    // any additional slots for T's initialization
    const ADDITIONAL_SLOTS_NEEDED: Slot = 1 + T::ADDITIONAL_SLOTS_NEEDED;
}

impl<'wren, T: Set<'wren>> Set<'wren> for [T] {
    unsafe fn send_to_vm(&self, vm: &mut RawVMContext<'wren>, slot: Slot) {
        send_iterator_to_vm(self.iter(), vm, slot);
    }
}

impl<'wren, T: Get<'wren>> Get<'wren> for Vec<T> {
    unsafe fn get_from_vm(vm: &mut RawVMContext<'wren>, slot: Slot) -> Self {
        // Store the next slot so we don't overwrite it's value
        // Or use the previous slot instead of juggling slots
        let (_store, item_slot) = if slot == 0 {
            (Some(store_slot(vm, slot + 1)), slot + 1)
        } else {
            (None, slot - 1)
        };

        let mut vec = vec![];

        let count = ffi::wrenGetListCount(vm.as_ptr(), slot);

        for i in 0..count {
            ffi::wrenGetListElement(vm.as_ptr(), slot, i, item_slot);
            vec.push(T::get_from_vm(vm, item_slot));
        }

        vec
    }
}

impl Value for CString {
    const ADDITIONAL_SLOTS_NEEDED: Slot = 0;
}

impl<'wren> Set<'wren> for CString {
    unsafe fn send_to_vm(&self, vm: &mut RawVMContext<'wren>, slot: Slot) {
        ffi::wrenSetSlotString(vm.as_ptr(), slot, self.as_ptr());
    }
}

unsafe fn send_string_to_vm<S: AsRef<str>>(vm: &mut RawVMContext, value: S, slot: Slot) {
    let str = value.as_ref();
    wren_sys::wrenSetSlotBytes(
        vm.as_ptr(),
        slot,
        str.as_ptr().cast(),
        // The len should always be valid
        str.len().try_into().expect("Invalid length for str"),
    );
}

impl Value for &str {
    const ADDITIONAL_SLOTS_NEEDED: Slot = 0;
}

impl<'wren> Set<'wren> for &str {
    unsafe fn send_to_vm(&self, vm: &mut RawVMContext<'wren>, slot: Slot) {
        send_string_to_vm(vm, self, slot);
    }
}

impl Value for String {
    const ADDITIONAL_SLOTS_NEEDED: Slot = 0;
}

impl<'wren> Set<'wren> for String {
    unsafe fn send_to_vm(&self, vm: &mut RawVMContext<'wren>, slot: Slot) {
        send_string_to_vm(vm, self, slot);
    }
}

impl<'wren> Get<'wren> for String {
    unsafe fn get_from_vm(vm: &mut RawVMContext<'wren>, slot: Slot) -> Self {
        let mut len = 0;
        let ptr = ffi::wrenGetSlotBytes(vm.as_ptr(), slot, &mut len).cast();
        let len = len.try_into().unwrap();
        let slice = std::slice::from_raw_parts(ptr, len);
        Self::from_utf8_lossy(slice).to_string()
    }
}

impl Value for f64 {
    const ADDITIONAL_SLOTS_NEEDED: Slot = 0;
}

impl<'wren> Set<'wren> for f64 {
    unsafe fn send_to_vm(&self, vm: &mut RawVMContext<'wren>, slot: Slot) {
        ffi::wrenSetSlotDouble(vm.as_ptr(), slot, *self);
    }
}

impl<'wren> Get<'wren> for f64 {
    unsafe fn get_from_vm(vm: &mut RawVMContext<'wren>, slot: Slot) -> Self {
        ffi::wrenGetSlotDouble(vm.as_ptr(), slot)
    }
}

impl Value for bool {
    const ADDITIONAL_SLOTS_NEEDED: Slot = 0;
}

impl<'wren> Set<'wren> for bool {
    unsafe fn send_to_vm(&self, vm: &mut RawVMContext<'wren>, slot: Slot) {
        ffi::wrenSetSlotBool(vm.as_ptr(), slot, *self);
    }
}

impl<'wren> Get<'wren> for bool {
    unsafe fn get_from_vm(vm: &mut RawVMContext<'wren>, slot: Slot) -> Self {
        let t = ffi::wrenGetSlotType(vm.as_ptr(), slot);
        match t {
            wren_sys::WrenType_WREN_TYPE_BOOL => ffi::wrenGetSlotBool(vm.as_ptr(), slot),
            wren_sys::WrenType_WREN_TYPE_NULL => false,
            _ => true,
        }
    }
}

pub trait SetArgs<'wren> {
    const REQUIRED_SLOTS: Slot;
    unsafe fn set_slots(&self, vm: &mut RawVMContext<'wren>);
    /// This fn should probably never be used directly since it only existed
    /// before required slots was a constant
    unsafe fn set_wren_stack_unchecked(&self, vm: &mut RawVMContext<'wren>, num_slots: Slot) {
        vm.ensure_slots(num_slots);
        self.set_slots(vm);
    }
    fn set_wren_stack(&self, vm: &mut RawVMContext<'wren>) {
        // This is guarenteed to be safe because we ensured that we had enough
        // slots for T using REQUIRED_SLOTS
        unsafe {
            self.set_wren_stack_unchecked(vm, Self::REQUIRED_SLOTS);
        }
    }
}

impl<'wren, T: Set<'wren> + ?Sized> SetArgs<'wren> for T {
    const REQUIRED_SLOTS: Slot = 1 + T::ADDITIONAL_SLOTS_NEEDED;
    unsafe fn set_slots(&self, vm: &mut RawVMContext<'wren>) {
        self.send_to_vm(vm, 0);
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
        impl<'wren, $( $xs: Set<'wren>, )*> SetArgs<'wren> for ($( &$xs, )*) {
            const REQUIRED_SLOTS: Slot =
                expand_required_slots!($( $xs ), *);


            unsafe fn set_slots(&self, vm: &mut RawVMContext<'wren>) {
                expand_set_slots!(self, vm, send_to_vm, $( $i ), *);
            }
        }
    };
}

impl_set_args!(T0 = 0, T1 = 1);
impl_set_args!(T0 = 0, T1 = 1, T2 = 2);
impl_set_args!(T0 = 0, T1 = 1, T2 = 2, T3 = 3);
impl_set_args!(T0 = 0, T1 = 1, T2 = 2, T3 = 3, T4 = 4);
impl_set_args!(T0 = 0, T1 = 1, T2 = 2, T3 = 3, T4 = 4, T5 = 5);
impl_set_args!(T0 = 0, T1 = 1, T2 = 2, T3 = 3, T4 = 4, T5 = 5, T6 = 6);

pub trait GetArgs<'wren> {
    unsafe fn get_slots(vm: &mut RawVMContext<'wren>) -> Self;
}

impl<'wren, T: Get<'wren>> GetArgs<'wren> for T {
    unsafe fn get_slots(vm: &mut RawVMContext<'wren>) -> Self {
        T::get_from_vm(vm, 0)
    }
}

macro_rules! impl_get_args {
    ($( $xs:ident = $i:tt ), *) => {
        impl<'wren, $( $xs: Get<'wren>, )*> GetArgs<'wren> for ($( $xs, )*) {
            unsafe fn get_slots(vm: &mut RawVMContext<'wren>) -> Self{
                (
                    $( $xs::get_from_vm(vm, $i) ), *
                )
            }
        }
    };
}

impl_get_args!(T0 = 0, T1 = 1);
impl_get_args!(T0 = 0, T1 = 1, T2 = 2);
impl_get_args!(T0 = 0, T1 = 1, T2 = 2, T3 = 3);
impl_get_args!(T0 = 0, T1 = 1, T2 = 2, T3 = 3, T4 = 4);
impl_get_args!(T0 = 0, T1 = 1, T2 = 2, T3 = 3, T4 = 4, T5 = 5);
impl_get_args!(T0 = 0, T1 = 1, T2 = 2, T3 = 3, T4 = 4, T5 = 5, T6 = 6);

#[cfg(test)]
mod test {
    use crate::wren::{Handle, Vm, VmUserData};

    use super::SetArgs;

    struct TestUserData;
    impl<'wren> VmUserData<'wren, Self> for TestUserData {}

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

    macro_rules! test_case {
        ($type:ty, $class:ident.$handle:ident($vm:ident) == $res:expr) => {
            assert!({
                let res: $type = super::super::util::make_call!($class.$handle($vm));
                res == $res
            })
        };
        ($type:ty, $class:ident.$handle:ident($vm:ident, $($args:expr),+ ) == $res:expr) => {
            assert!({
                let res: $type = super::super::util::make_call!($class.$handle($vm, $($args),+ ));
                res == $res
            })
        };
    }

    fn create_test_vm<'wren>(source: &str) -> (Vm<'wren, TestUserData>, Handle<'wren>) {
        let mut vm = Vm::new(TestUserData);

        let vmptr = vm.get_context();
        vmptr
            .interpret("<test>", source)
            .expect("Code should run successfully");

        vmptr.ensure_slots(1);
        let class = unsafe { vmptr.get_variable_unchecked("<test>", "Test", 0) };

        (vm, class)
    }

    // Test that all values other than null and false are falsy
    #[test]
    #[allow(non_snake_case)]
    fn test_bool() {
        use crate::wren::make_call_handle;
        let source = "class Test {
                static returnTrue() { true }
                static returnFalse() { false }
                static returnNull() { null }
                static returnValue(value) { value }
                static returnNegate(value) { !value }
            }";

        let (mut vm, Test) = create_test_vm(source);
        let context = vm.get_context();
        let returnTrue = make_call_handle!(context, "returnTrue()");
        let returnFalse = make_call_handle!(context, "returnFalse()");
        let returnNull = make_call_handle!(context, "returnNull()");
        let returnValue = make_call_handle!(context, "returnValue(_)");
        let returnNegate = make_call_handle!(context, "returnNegate(_)");

        unsafe {
            // False cases
            test_case!(bool, Test.returnNull(context) == false);
            test_case!(bool, Test.returnFalse(context) == false);
            test_case!(bool, Test.returnValue(context, false) == false);
            test_case!(bool, Test.returnNegate(context, true) == false);
            test_case!(bool, Test.returnNegate(context, "") == false);

            // True cases
            test_case!(bool, Test.returnTrue(context) == true);
            test_case!(bool, Test.returnValue(context, true) == true);
            test_case!(bool, Test.returnNegate(context, false) == true);
            test_case!(bool, Test.returnValue(context, "") == true);
            test_case!(bool, Test.returnValue(context, "Test") == true);
            test_case!(bool, Test.returnValue(context, Test) == true);
            test_case!(bool, Test.returnValue(context, vec![1.0]) == true);
            test_case!(bool, Test.returnValue(context, 1.0) == true);
        }
    }
}
