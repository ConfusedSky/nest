use num_bigint::ToBigInt;
use wren::{ForeignClassMethods, WrenType};
use wren_macros::foreign_method;

use crate::{source_file, Context};

use super::{Class, Module};
use wren::ForeignClass;

pub fn init_module<'wren>() -> Module<'wren> {
    let mut bigint_class = Class::new();
    bigint_class.foreign_class_methods = Some(ForeignClassMethods::new::<BigInt>());
    bigint_class
        .methods
        .insert("setValue(_)", foreign_set_value);
    bigint_class.methods.insert("toString", foreign_to_string);

    let mut test_marker_class = Class::new();
    test_marker_class.foreign_class_methods = Some(ForeignClassMethods::new::<TestMarker>());

    let mut module = Module::new(source_file!("bigint.wren"));
    module.classes.insert("BigInt", bigint_class);
    module.classes.insert("Test", test_marker_class);

    module
}

#[derive(Default, Debug)]
struct TestMarker(String);

#[derive(Default, Debug)]
struct BigInt {
    data: num_bigint::BigInt,
}

enum Data<'wren> {
    BigInt(ForeignClass<'wren, BigInt>),
    Integer(i64),
}

#[foreign_method]
fn set_value(context: &mut Context, mut this: ForeignClass<BigInt>) -> Result<(), String> {
    internal_set_value(context, &mut this, "setValue")
}

#[foreign_method]
fn to_string(this: ForeignClass<BigInt>) -> String {
    this.data.to_string()
}

#[allow(unsafe_code)]
fn get_data<'wren>(context: &'wren mut Context, method: &'wren str) -> Result<Data<'wren>, String> {
    use wren::GetValue;
    // TODO: Implement better reflection so this error message is possible on the
    // rust side
    // "BigInt.%(method) expects a BigInt or an Integer got %(value): %(value.type)"
    let error = format!("BigInt.{method} expects a BigInt or an Integer");
    let slot_type = unsafe { context.get_slot_type(1) };
    match slot_type {
        WrenType::Num => {
            let slot = unsafe { f64::get_slot_unchecked(context, 1, WrenType::Num) };
            // We only take integers here
            #[allow(clippy::cast_possible_truncation)]
            if (slot.trunc() - slot).abs() < f64::EPSILON {
                Ok(Data::Integer(slot as i64))
            } else {
                Err(error)
            }
        }
        WrenType::Foreign => {
            let slot = unsafe { ForeignClass::<BigInt>::try_get_slot_unchecked(context, 1) };
            let slot = slot.map_err(|_| error)?;
            Ok(Data::BigInt(slot))
        }
        _ => Err(error),
    }
}

fn internal_set_value(
    context: &mut Context,
    this: &mut ForeignClass<BigInt>,
    method: &str,
) -> Result<(), String> {
    let data = get_data(context, method)?;

    match data {
        Data::BigInt(i) => this.data = i.data.clone(),
        // This shouldn't error since we checked to make sure the value
        // is an integer earlier
        Data::Integer(i) => this.data = i.to_bigint().unwrap(),
    }

    Ok(())
}
