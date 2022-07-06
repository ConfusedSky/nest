#![allow(clippy::module_name_repetitions)]

use std::{ffi::CString, ptr::NonNull};

use wren_sys as ffi;

use super::{
    context::{Foreign, Location, Native, Raw as RawContext, UnknownLocation},
    Handle, Slot,
};
use enumflags2::{bitflags, make_bitflags, BitFlags};

#[derive(Debug, PartialEq)]
pub enum TryGetError<'wren> {
    // If type is incompatible there is still an option to
    // retreive the handle for another reason
    IncompatibleType(Option<Handle<'wren>>),
    NoAvailableSlot,
}

pub type TryGetResult<'wren, T> = Result<T, TryGetError<'wren>>;

#[bitflags]
#[repr(u8)]
#[derive(PartialEq, Debug, Copy, Clone)]
pub enum WrenType {
    Bool,
    Num,
    Foreign,
    List,
    Map,
    Null,
    String,
    Unknown,
}

impl From<ffi::WrenType> for WrenType {
    fn from(other: ffi::WrenType) -> Self {
        match other {
            ffi::WrenType_WREN_TYPE_BOOL => Self::Bool,
            ffi::WrenType_WREN_TYPE_NUM => Self::Num,
            ffi::WrenType_WREN_TYPE_FOREIGN => Self::Foreign,
            ffi::WrenType_WREN_TYPE_LIST => Self::List,
            ffi::WrenType_WREN_TYPE_MAP => Self::Map,
            ffi::WrenType_WREN_TYPE_NULL => Self::Null,
            ffi::WrenType_WREN_TYPE_STRING => Self::String,
            _ => Self::Unknown,
        }
    }
}

struct SlotStorage<'wren> {
    vm: RawContext<'wren, UnknownLocation>,
    slot: Slot,
    handle: Handle<'wren>,
}

impl<'wren> Drop for SlotStorage<'wren> {
    fn drop(&mut self) {
        // Slots are counted from 0 so to store this slot
        // we must ensure slot + 1 exist
        unsafe {
            self.vm.ensure_slots(self.slot + 1);
            self.handle.set_slot(&mut self.vm, self.slot);
        }
    }
}

unsafe fn store_slot<'wren, L: Location>(
    vm: &mut RawContext<'wren, L>,
    slot: Slot,
) -> SlotStorage<'wren> {
    let vm = vm.as_unknown_mut();
    // Same idea as above for the drop function
    vm.ensure_slots(slot + 1);
    // Here we are just storing a handle so we don't care too much
    // where it comes from
    let handle = Handle::get_slot_raw(vm, slot, WrenType::Unknown);
    SlotStorage {
        vm: vm.clone(),
        slot,
        handle,
    }
}

pub trait SetValue<'wren, L: Location> {
    /// Number of additional slots that need to be allocated to use this
    const REQUIRED_SLOTS: Slot;
    unsafe fn set_slot(&self, vm: &mut RawContext<'wren, L>, slot: Slot);
}

pub trait GetValue<'wren, L: Location> {
    const COMPATIBLE_TYPES: BitFlags<WrenType>;
    /// NOTE THE IMPLEMENTATION OF GET FROM VM SHOULD BE SAFE REGARDLESS
    /// OF WHAT IS IN THE SLOT
    /// `get_from_vm` is unsafe because it's not guarenteed that
    /// the slot is a valid slot
    unsafe fn get_slot_raw(vm: &mut RawContext<'wren, L>, slot: Slot, slot_type: WrenType) -> Self;
    unsafe fn get_slot_unchecked(vm: &mut RawContext<'wren, L>, slot: Slot) -> Self
    where
        Self: Sized,
    {
        Self::get_slot_raw(vm, slot, vm.get_slot_type(slot))
    }
    unsafe fn try_get_slot_raw(
        vm: &mut RawContext<'wren, L>,
        slot: Slot,
        slot_type: WrenType,
        get_handle: bool,
    ) -> TryGetResult<'wren, Self>
    where
        Self: Sized,
    {
        if Self::COMPATIBLE_TYPES.contains(slot_type) {
            Ok(Self::get_slot_raw(vm, slot, slot_type))
        } else {
            Err(TryGetError::IncompatibleType(if get_handle {
                Some(Handle::get_slot_unchecked(vm, slot))
            } else {
                None
            }))
        }
    }
}

// () is implemented to allow skipping slots
// and to set send null to the vm
impl<'wren, L: Location> SetValue<'wren, L> for () {
    const REQUIRED_SLOTS: Slot = 1;
    unsafe fn set_slot(&self, vm: &mut RawContext<'wren, L>, slot: Slot) {
        ffi::wrenSetSlotNull(vm.as_ptr(), slot);
    }
}

impl<'wren, L: Location> GetValue<'wren, L> for () {
    const COMPATIBLE_TYPES: BitFlags<WrenType> = BitFlags::ALL;
    unsafe fn get_slot_raw(
        _vm: &mut RawContext<'wren, L>,
        _slot: Slot,
        _slot_type: WrenType,
    ) -> Self {
    }
    unsafe fn get_slot_unchecked(_vm: &mut RawContext<'wren, L>, _slot: Slot) -> Self
    where
        Self: Sized,
    {
    }
    unsafe fn try_get_slot_raw(
        _vm: &mut RawContext<'wren, L>,
        _slot: Slot,
        _slot_type: WrenType,
        _get_handle: bool,
    ) -> TryGetResult<'wren, Self>
    where
        Self: Sized,
    {
        Ok(())
    }
}

impl<'wren, L: Location, T: SetValue<'wren, L>> SetValue<'wren, L> for &T {
    const REQUIRED_SLOTS: Slot = T::REQUIRED_SLOTS;
    unsafe fn set_slot(&self, vm: &mut RawContext<'wren, L>, slot: Slot) {
        (*self).set_slot(vm, slot);
    }
}

impl<'wren, L: Location> SetValue<'wren, L> for Handle<'wren> {
    const REQUIRED_SLOTS: Slot = 1;
    unsafe fn set_slot(&self, vm: &mut RawContext<'wren, L>, slot: Slot) {
        ffi::wrenSetSlotHandle(vm.as_ptr(), slot, self.as_ptr());
    }
}
impl<'wren, L: Location> GetValue<'wren, L> for Handle<'wren> {
    const COMPATIBLE_TYPES: BitFlags<WrenType> = BitFlags::ALL;
    // We are always able to Get<'wren> a handle from wren
    unsafe fn get_slot_raw(
        vm: &mut RawContext<'wren, L>,
        slot: Slot,
        _slot_type: WrenType,
    ) -> Self {
        let handle = ffi::wrenGetSlotHandle(vm.as_ptr(), slot);
        Self::new_unchecked(vm, NonNull::new_unchecked(handle))
    }
    unsafe fn get_slot_unchecked(vm: &mut RawContext<'wren, L>, slot: Slot) -> Self
    where
        Self: Sized,
    {
        Self::get_slot_raw(vm, slot, WrenType::Unknown)
    }
}

unsafe fn send_iterator_to_vm<'wren, L: Location, T: SetValue<'wren, L>, I: Iterator<Item = T>>(
    iterator: I,
    vm: &mut RawContext<'wren, L>,
    slot: Slot,
) {
    let mut list = None;
    ffi::wrenSetSlotNewList(vm.as_ptr(), slot);
    // Store the next slot so we don't overwrite it's value
    // Or use the previous slot instead of juggling slots
    let (_store, item_slot) = if slot == 0 {
        //  We should store the list in a handle as well just
        // in case send_to_vm overwrites [slot]
        list = Some(Handle::get_slot_unchecked(vm, slot));
        (Some(store_slot(vm, slot + 1)), slot + 1)
    } else {
        (None, slot - 1)
    };

    for value in iterator {
        value.set_slot(vm, item_slot);
        // If we had to store the list earlier
        // then it needs to be sent to the vm again
        if let Some(list) = &list {
            list.set_slot(vm, slot);
        }
        ffi::wrenInsertInList(vm.as_ptr(), slot, -1, item_slot);
    }
}

impl<'wren, L: Location, T: SetValue<'wren, L>> SetValue<'wren, L> for Vec<T> {
    // This needs at least one for moving values into the wren list as well as
    // any additional slots for T's initialization
    const REQUIRED_SLOTS: Slot = 1 + T::REQUIRED_SLOTS;
    unsafe fn set_slot(&self, vm: &mut RawContext<'wren, L>, slot: Slot) {
        send_iterator_to_vm(self.iter(), vm, slot);
    }
}

impl<'wren, L: Location, T: GetValue<'wren, L>> GetValue<'wren, L> for Vec<T> {
    const COMPATIBLE_TYPES: BitFlags<WrenType> = make_bitflags!(WrenType::{List});
    unsafe fn get_slot_raw(vm: &mut RawContext<'wren, L>, slot: Slot, slot_type: WrenType) -> Self {
        // Store the next slot so we don't overwrite it's value
        // Or use the previous slot instead of juggling slots
        let (_store, item_slot) = if slot == 0 {
            // Make sure the slot that we are storing actually exits
            (Some(store_slot(vm, slot + 1)), slot + 1)
        } else {
            (None, slot - 1)
        };

        let mut vec = vec![];

        // TODO: Handle this better than just returning an empty vec
        // if cfg!(debug_assertions) {
        // assert!(ty == WrenType::List, "Unable to get non list value as Vec");
        // }

        if slot_type == WrenType::List {
            let count = ffi::wrenGetListCount(vm.as_ptr(), slot);

            for i in 0..count {
                ffi::wrenGetListElement(vm.as_ptr(), slot, i, item_slot);
                // Wren arrays can be mixed items
                // So this will default a couple of the items presumably
                let item_type = vm.get_slot_type(item_slot);
                vec.push(T::get_slot_raw(vm, item_slot, item_type));
            }
        }

        vec
    }
}

impl<'wren, L: Location, T: SetValue<'wren, L>> SetValue<'wren, L> for [T] {
    // This needs at least one for moving values into the wren list as well as
    // any additional slots for T's initialization
    const REQUIRED_SLOTS: Slot = 1 + T::REQUIRED_SLOTS;
    unsafe fn set_slot(&self, vm: &mut RawContext<'wren, L>, slot: Slot) {
        send_iterator_to_vm(self.iter(), vm, slot);
    }
}

impl<'wren, L: Location> SetValue<'wren, L> for CString {
    const REQUIRED_SLOTS: Slot = 1;
    unsafe fn set_slot(&self, vm: &mut RawContext<'wren, L>, slot: Slot) {
        ffi::wrenSetSlotString(vm.as_ptr(), slot, self.as_ptr());
    }
}

unsafe fn send_string_to_vm<S: AsRef<str>, L: Location>(
    vm: &mut RawContext<L>,
    value: S,
    slot: Slot,
) {
    let str = value.as_ref();
    wren_sys::wrenSetSlotBytes(
        vm.as_ptr(),
        slot,
        str.as_ptr().cast(),
        // The len should always be valid
        str.len().try_into().expect("Invalid length for str"),
    );
}

impl<'wren, L: Location> SetValue<'wren, L> for &str {
    const REQUIRED_SLOTS: Slot = 1;
    unsafe fn set_slot(&self, vm: &mut RawContext<'wren, L>, slot: Slot) {
        send_string_to_vm(vm, self, slot);
    }
}

unsafe fn generic_get_string<L: Location>(
    vm: &mut RawContext<L>,
    slot: Slot,
    slot_type: WrenType,
) -> Option<String> {
    match slot_type {
        WrenType::Bool => Some(ffi::wrenGetSlotBool(vm.as_ptr(), slot).to_string()),
        WrenType::Null => Some("null".to_string()),
        WrenType::String => {
            let mut len = 0;
            let ptr = ffi::wrenGetSlotBytes(vm.as_ptr(), slot, &mut len).cast();
            let len = len.try_into().unwrap();
            let slice = std::slice::from_raw_parts(ptr, len);
            Some(String::from_utf8_lossy(slice).to_string())
        }
        _ => None,
    }
}

impl<'wren, L: Location> SetValue<'wren, L> for String {
    const REQUIRED_SLOTS: Slot = 1;
    unsafe fn set_slot(&self, vm: &mut RawContext<'wren, L>, slot: Slot) {
        send_string_to_vm(vm, self, slot);
    }
}

impl<'wren> GetValue<'wren, Native> for String {
    const COMPATIBLE_TYPES: BitFlags<WrenType> = BitFlags::ALL;
    unsafe fn get_slot_raw(
        vm: &mut RawContext<'wren, Native>,
        slot: Slot,
        slot_type: WrenType,
    ) -> Self {
        generic_get_string(vm, slot, slot_type).unwrap_or_else(|| {
            // Fall back to calling to string on the returned object
            // ONLY WORKS IN NATIVE CODE
            let system_methods = vm.get_system_methods();
            let handle = Handle::get_slot_raw(vm, slot, WrenType::Unknown);
            let to_string = &system_methods.object_to_string;

            // NOTE: this might break the native stack so it might need to be
            // saved off first
            // Note this shouldn't recurse because the second call
            // will always be called on a "real" string
            vm.call(&handle, to_string, &())
                .expect("toString should never fail on a valid wren handle")
        })
    }
}

impl<'wren> GetValue<'wren, Foreign> for String {
    const COMPATIBLE_TYPES: BitFlags<WrenType> = make_bitflags!(WrenType::{Bool | Null | String});
    unsafe fn get_slot_raw(
        vm: &mut RawContext<'wren, Foreign>,
        slot: Slot,
        slot_type: WrenType,
    ) -> Self {
        generic_get_string(vm, slot, slot_type).expect("Should not be called on incompatible type")
    }
}

impl<'wren, L: Location> SetValue<'wren, L> for f64 {
    const REQUIRED_SLOTS: Slot = 1;
    unsafe fn set_slot(&self, vm: &mut RawContext<'wren, L>, slot: Slot) {
        ffi::wrenSetSlotDouble(vm.as_ptr(), slot, *self);
    }
}

impl<'wren, L: Location> GetValue<'wren, L> for f64 {
    const COMPATIBLE_TYPES: BitFlags<WrenType> = make_bitflags!(WrenType::{Num});
    unsafe fn get_slot_raw(vm: &mut RawContext<'wren, L>, slot: Slot, slot_type: WrenType) -> Self {
        if WrenType::Num == slot_type {
            ffi::wrenGetSlotDouble(vm.as_ptr(), slot)
        } else {
            Self::NAN
        }
    }
}

impl<'wren, L: Location> SetValue<'wren, L> for bool {
    const REQUIRED_SLOTS: Slot = 1;
    unsafe fn set_slot(&self, vm: &mut RawContext<'wren, L>, slot: Slot) {
        ffi::wrenSetSlotBool(vm.as_ptr(), slot, *self);
    }
}

impl<'wren, L: Location> GetValue<'wren, L> for bool {
    const COMPATIBLE_TYPES: BitFlags<WrenType> = BitFlags::ALL;
    unsafe fn get_slot_raw(vm: &mut RawContext<'wren, L>, slot: Slot, slot_type: WrenType) -> Self {
        match slot_type {
            WrenType::Bool => ffi::wrenGetSlotBool(vm.as_ptr(), slot),
            WrenType::Null => false,
            _ => true,
        }
    }
}

pub trait SetArgs<'wren, L: Location> {
    const COUNT: usize;
    const TOTAL_REQUIRED_SLOTS: Slot;
    unsafe fn set_slots(&self, vm: &mut RawContext<'wren, L>, offset: u16);
    /// This fn should probably never be used directly since it only existed
    /// before required slots was a constant
    unsafe fn set_wren_stack_unchecked(
        &self,
        vm: &mut RawContext<'wren, L>,
        num_slots: Slot,
        offset: u16,
    ) {
        vm.ensure_slots(num_slots + Slot::from(offset));
        self.set_slots(vm, offset);
    }
    /// Sets the values on the [vm]'s stack
    /// also accepts a [offset] parameter that allows the items to be shifted up
    /// to allow other items to be passed beforehand
    fn set_wren_stack(&self, vm: &mut RawContext<'wren, L>, offset: u16) {
        // This is guarenteed to be safe because we ensured that we had enough
        // slots for T using TOTAL_REQUIRED_SLOTS
        unsafe {
            self.set_wren_stack_unchecked(vm, Self::TOTAL_REQUIRED_SLOTS, offset);
        }
    }
}

impl<'wren, L: Location> SetArgs<'wren, L> for () {
    const COUNT: usize = 0;
    const TOTAL_REQUIRED_SLOTS: Slot = 1;
    unsafe fn set_slots(&self, _: &mut RawContext<'wren, L>, _: u16) {
        // An empty arg list should do nothing
        // This currently sends a null
        // ().send_to_vm(vm, Slot::from(offset));
    }
}

impl<'wren, L: Location, T: SetValue<'wren, L> + ?Sized> SetArgs<'wren, L> for &T {
    const COUNT: usize = 1;
    const TOTAL_REQUIRED_SLOTS: Slot = T::REQUIRED_SLOTS;
    unsafe fn set_slots(&self, vm: &mut RawContext<'wren, L>, offset: u16) {
        self.set_slot(vm, Slot::from(offset));
    }
}

const fn _const_max_helper(a: Slot, b: Slot) -> Slot {
    [a, b][(a < b) as usize]
}

macro_rules! expand_TOTAL_REQUIRED_SLOTS {
    (@step $i:expr, $x:ty) => (<$x>::REQUIRED_SLOTS + $i);
    (@step $i:expr, $x:ty, $($y:ty),+ $(,)?) => (
        _const_max_helper(
            <$x>::REQUIRED_SLOTS + $i,
            expand_TOTAL_REQUIRED_SLOTS!(@step $i + 1, $($y),+),
        )
    );
    ($x:ty, $($y:ty),+ $(,)?) => (
        expand_TOTAL_REQUIRED_SLOTS!(@step 0i32, $x, $($y),+)
    )
}

macro_rules! expand_set_slots {
    ($self:ident, $vm:ident, $method:ident, $offset:expr, $i:tt $(, $($xs:tt),+)?) => {
        $(expand_set_slots!($self, $vm, $method, $offset, $( $xs ), +);)?
        $self.$i.$method($vm, $i + $offset as Slot);
    };
}

macro_rules! count {
    () => (0usize);
    ( $x:tt $($xs:tt)* ) => (1usize + count!($($xs)*));
}

macro_rules! impl_set_args_meta {
    ($location:ty, $( $xs:ident = $i:tt ), *) => {
        impl<'wren, $( $xs: SetValue<'wren, $location>, )*> SetArgs<'wren, $location> for ($( &$xs, )*) {
            const COUNT: usize = count!( $( $xs ) * );
            const TOTAL_REQUIRED_SLOTS: Slot =
                expand_TOTAL_REQUIRED_SLOTS!($( $xs ), *);


            unsafe fn set_slots(&self, vm: &mut RawContext<'wren, $location>, offset: u16) {
                // Expansion happens in reverse order so previous slots
                // can be used when scratch slots are needed
                expand_set_slots!(self, vm, set_slot, offset, $( $i ), *);
            }
        }
    };
}

macro_rules! impl_set_args {
    ($( $xs:ident = $i:tt ), *) => {
        impl_set_args_meta!(Native, $( $xs = $i ), *);
        impl_set_args_meta!(Foreign, $( $xs = $i ), *);
    };
}

impl_set_args!(T0 = 0, T1 = 1);
impl_set_args!(T0 = 0, T1 = 1, T2 = 2);
impl_set_args!(T0 = 0, T1 = 1, T2 = 2, T3 = 3);
impl_set_args!(T0 = 0, T1 = 1, T2 = 2, T3 = 3, T4 = 4);
impl_set_args!(T0 = 0, T1 = 1, T2 = 2, T3 = 3, T4 = 4, T5 = 5);
impl_set_args!(T0 = 0, T1 = 1, T2 = 2, T3 = 3, T4 = 4, T5 = 5, T6 = 6);

pub trait GetArgs<'wren, L: Location> {
    type TryGetTarget;

    unsafe fn get_slots_unchecked(vm: &mut RawContext<'wren, L>) -> Self;
    fn try_get_slots(vm: &mut RawContext<'wren, L>, get_handles: bool) -> Self::TryGetTarget;
}

impl<'wren, L: Location, T: GetValue<'wren, L>> GetArgs<'wren, L> for T {
    type TryGetTarget = TryGetResult<'wren, T>;
    unsafe fn get_slots_unchecked(vm: &mut RawContext<'wren, L>) -> Self {
        T::get_slot_unchecked(vm, 0)
    }
    fn try_get_slots(vm: &mut RawContext<'wren, L>, get_handles: bool) -> Self::TryGetTarget {
        // Hack to make sure () does nothing
        // Should compile out since it's a const function on a type
        if std::mem::size_of::<T>() == 0 {
            unsafe { T::try_get_slot_raw(vm, 0, WrenType::Unknown, false) }
        } else {
            if vm.get_slot_count() < 1 {
                return Err(TryGetError::NoAvailableSlot);
            }

            let slot_type = unsafe { vm.get_slot_type(0) };
            unsafe { T::try_get_slot_raw(vm, 0, slot_type, get_handles) }
        }
    }
}

macro_rules! impl_get_args_meta {
    ($location:ty, $( $xs:ident = $i:tt ), *) => {
        impl<'wren, $( $xs: GetValue<'wren, $location>, )*> GetArgs<'wren, $location> for ($( $xs, )*) {
            type TryGetTarget = ($(TryGetResult<'wren, $xs>),*);
            unsafe fn get_slots_unchecked(vm: &mut RawContext<'wren, $location>) -> Self {
                (
                    // Expansion happens in forward order so
                    // Previous slots can be reused for reads
                    $( $xs::get_slot_unchecked(vm, $i) ), *
                )
            }
            fn try_get_slots(vm: &mut RawContext<'wren, $location>, get_handles: bool) -> Self::TryGetTarget {
                let stack_types = vm.get_stack_types();

                (
                    $({
                        if stack_types.len() < $i + 1 {
                            Err(TryGetError::NoAvailableSlot)
                        } else {
                            unsafe {
                                $xs::try_get_slot_raw(vm, $i, stack_types[$i], get_handles)
                            }
                        }
                    }), *
                )
            }
        }
    };
}

macro_rules! impl_get_args {
    ($( $xs:ident = $i:tt ), *) => {
        impl_get_args_meta!(Native, $( $xs = $i ), *);
        impl_get_args_meta!(Foreign, $( $xs = $i ), *);
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
    use crate::{
        call_test_case, call_test_case2,
        wren::{
            context::{Foreign, Native},
            test::create_test_vm,
            value::TryGetError,
        },
    };

    use super::SetArgs;

    #[test]
    fn test_slot_size() {
        macro_rules! meta_test {
            ($location:ty) => {{
                type L = $location;
                assert_eq!(<&f64 as SetArgs<L>>::TOTAL_REQUIRED_SLOTS, 1);
                assert_eq!(<(&f64, &f64) as SetArgs<L>>::TOTAL_REQUIRED_SLOTS, 2);
                assert_eq!(
                    <(&Vec<Vec<f64>>, &f64) as SetArgs<L>>::TOTAL_REQUIRED_SLOTS,
                    3
                );
                assert_eq!(
                    <(&f64, &Vec<Vec<f64>>) as SetArgs<L>>::TOTAL_REQUIRED_SLOTS,
                    4
                );
                assert_eq!(<(&f64, &f64, &f64) as SetArgs<L>>::TOTAL_REQUIRED_SLOTS, 3);
                assert_eq!(
                    <(&f64, &f64, &f64, &f64) as SetArgs<L>>::TOTAL_REQUIRED_SLOTS,
                    4
                );
            }};
        }
        meta_test!(Foreign);
        meta_test!(Native);
    }

    // Test that all values other than null and false are falsy
    #[test]
    #[allow(non_snake_case)]
    fn test_bool() {
        let source = "class Test {
                static returnTrue { true }
                static returnTrue() { true }
                static returnFalse() { false }
                static returnNull() { null }
                static returnValue(value) { value }
                static returnNegate(value) { !value }
            }";

        let (mut vm, Test) = create_test_vm(source, |_| {});
        let context = vm.get_context();

        call_test_case2!(context<bool> {
            // False cases
            Test.returnNull() == Ok(false)
            Test.returnFalse() == Ok(false)
            Test.returnValue(false) == Ok(false)
            Test.returnNegate(true) == Ok(false)
            Test.returnNegate("") == Ok(false)
            // True Cases
            Test.returnTrue == Ok(true)
            Test.returnTrue() == Ok(true)
            Test.returnValue(true) == Ok(true)
            Test.returnNegate(false) == Ok(true)
            Test.returnValue("") == Ok(true)
            Test.returnValue("Test") == Ok(true)
            Test.returnValue(Test) == Ok(true)
            Test.returnValue(vec![1.0]) == Ok(true)
            Test.returnValue(1.0) == Ok(true)
        });
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_string() {
        let source = "class Test {
                static returnTrue { true }
                static returnTrue() { true }
                static returnFalse() { false }
                static returnNull() { null }
                static returnValue(value) { value }
                static sendMulti(value, value2) { value.toString + value2.toString }
                static returnMap {
                    var m = {}
                    m[\"test\"] = 1
                    m[15] = Test
                    return m
                }
            }";

        let (mut vm, Test) = create_test_vm(source, |_| {});
        let context = vm.get_context();

        call_test_case2!(context<String> {
            Test.returnNull() == Ok("null".to_string())
            Test.returnFalse() == Ok("false".to_string())
            Test.returnValue(false) == Ok("false".to_string())
            Test.returnTrue == Ok("true".to_string())
            Test.returnTrue() == Ok("true".to_string())
            Test.returnValue("") == Ok("".to_string())
            Test.returnValue("Test") == Ok("Test".to_string())
            Test.returnValue("Test".to_string()) == Ok("Test".to_string())
            Test.returnValue(Test) == Ok("Test".to_string())
            Test.returnValue(vec![1.0]) == Ok("[1]".to_string())
            Test.returnValue(vec!["1.0", "Other"]) == Ok("[1.0, Other]".to_string())
            Test.returnValue(1.0) == Ok("1".to_string())
            Test.returnMap == Ok("{test: 1, 15: Test}".to_string())
            Test.sendMulti("Test", vec![1.0]) == Ok("Test[1]".to_string())
            Test.sendMulti(vec!["One Two"], "Test") == Ok("[One Two]Test".to_string())
        });
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_vec() {
        let source = "class Test {
                static returnTrue { true }
                static returnTrue() { true }
                static returnFalse() { false }
                static returnNull() { null }
                static returnValue(value) { value }
                static nestedArray { [[[1]]] }
                static sendMulti(value, value2) { value.toString + value2.toString }
                static returnMap {
                    var m = {}
                    m[\"test\"] = 1
                    m[15] = Test
                    return m
                }
            }";

        let (mut vm, Test) = create_test_vm(source, |_| {});
        let context = vm.get_context();

        call_test_case2!(context<Vec<String>> {
            Test.returnNull() == Err(TryGetError::IncompatibleType(None).into())
            Test.returnFalse() == Err(TryGetError::IncompatibleType(None).into())
            Test.returnValue(false) == Err(TryGetError::IncompatibleType(None).into())
            Test.returnTrue == Err(TryGetError::IncompatibleType(None).into())
            Test.returnTrue() == Err(TryGetError::IncompatibleType(None).into())
            Test.returnValue("") == Err(TryGetError::IncompatibleType(None).into())
            Test.returnValue("Test") == Err(TryGetError::IncompatibleType(None).into())
            Test.returnValue("Test".to_string()) == Err(TryGetError::IncompatibleType(None).into())
            Test.returnValue(Test) == Err(TryGetError::IncompatibleType(None).into())
            Test.returnValue(vec![1.0]) == Ok(vec!["1".to_string()])
            Test.returnValue(vec!["1.0", "Other"]) == Ok(vec!["1.0".to_string(), "Other".to_string()])
            Test.returnValue(1.0) == Err(TryGetError::IncompatibleType(None).into())
            Test.returnMap == Err(TryGetError::IncompatibleType(None).into())
            Test.sendMulti("Test", vec![1.0]) == Err(TryGetError::IncompatibleType(None).into())
            Test.sendMulti(vec!["One Two"], "Test") == Err(TryGetError::IncompatibleType(None).into())
            Test.returnValue( vec![vec![vec!["1.0", "Other"]]]) == Ok(vec!["[[1.0, Other]]".to_string()])
        });
        call_test_case!(Vec<Vec<Vec<f64>>>, context { Test.nestedArray } == Ok(vec![vec![vec![1.0_f64]]]));
        call_test_case!(Vec<Vec<String>>, context { Test.returnValue(vec![vec!["1.0", "Other"]]) } == Ok(vec![vec!["1.0".to_string(), "Other".to_string()]]));
    }
}
