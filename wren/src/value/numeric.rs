use crate::{
    context::{Location, Raw as RawContext},
    GetValue, SetValue, Slot, WrenType,
};
use enumflags2::{make_bitflags, BitFlags};
use wren_sys as ffi;

impl<'wren, L: Location> SetValue<'wren, L> for f64 {
    unsafe fn set_slot(&self, vm: &mut RawContext<'wren, L>, slot: Slot) {
        ffi::wrenSetSlotDouble(vm.as_ptr(), slot, *self);
    }
}

impl<'wren, L: Location> GetValue<'wren, L> for f64 {
    const COMPATIBLE_TYPES: BitFlags<WrenType> = make_bitflags!(WrenType::{Num});
    unsafe fn get_slot_unchecked(
        vm: &mut RawContext<'wren, L>,
        slot: Slot,
        slot_type: WrenType,
    ) -> Self {
        if WrenType::Num == slot_type {
            ffi::wrenGetSlotDouble(vm.as_ptr(), slot)
        } else {
            Self::NAN
        }
    }
}
