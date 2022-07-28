#![allow(unsafe_code)]

use num_bigint::{BigInt, ToBigInt};
use wren::{
    context::{Foreign, NoTypeInfo},
    ForeignClassMethods, GetValue, Handle, WrenType,
};
use wren_macros::foreign_method;

use crate::{source_file, Context, MyUserData};

use super::{Class, Module};
use wren::ForeignClass;

pub fn init_module<'wren>() -> Module<'wren> {
    let mut bigint_class = Class::new();
    bigint_class.foreign_class_methods = Some(ForeignClassMethods::new::<BigInt>());
    bigint_class
        .methods
        .insert("setValue(_)", foreign_set_value);
    bigint_class.methods.insert("toString", foreign_to_string);
    bigint_class.methods.insert("+(_)", add);

    let mut test_marker_class = Class::new();
    test_marker_class.foreign_class_methods = Some(ForeignClassMethods::new::<TestMarker>());

    let mut module = Module::new(source_file!("bigint.wren"));
    module.classes.insert("BigInt", bigint_class);
    module.classes.insert("Test", test_marker_class);

    module
}

pub struct BigIntModule<'wren>(Handle<'wren>);

#[derive(Default, Debug)]
struct TestMarker(String);

enum Data<'wren> {
    BigInt(ForeignClass<'wren, BigInt>),
    Integer(i64),
}

#[foreign_method]
fn set_value<'wren>(
    context: &mut Context<'wren>,
    mut this: ForeignClass<'wren, BigInt>,
) -> Result<(), String> {
    internal_set_value(context, &mut this, "setValue")
}

fn add(mut context: Context) {
    let this = unsafe { ForeignClass::<BigInt>::get_slot(&mut context, 0) };
    let result = {
        match get_data(&mut context, "+(_)") {
            Ok(Data::BigInt(ref i)) => this.as_ref() + i.as_ref(),
            Ok(Data::Integer(i)) => this.as_ref() + i,
            Err(s) => {
                context.abort_fiber(s);
                return;
            }
        }
    };

    let (user_data, context) = context.get_user_data_mut_with_context();
    let class_handle = get_class_handle(context, user_data);
    match &class_handle {
        Some(data) => unsafe {
            context.create_new_foreign(data, result);
        },
        None => context.abort_fiber("Could not load the BigInt class"),
    }
}

#[foreign_method]
fn to_string(this: ForeignClass<BigInt>) -> String {
    this.to_string()
}

fn get_data<'wren>(context: &mut Context<'wren>, method: &str) -> Result<Data<'wren>, String> {
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

fn internal_set_value<'wren>(
    context: &mut Context<'wren>,
    this: &mut ForeignClass<'wren, BigInt>,
    method: &str,
) -> Result<(), String> {
    let data = get_data(context, method)?;

    match data {
        Data::BigInt(i) => *this.as_mut() = i.as_ref().clone(),
        // This shouldn't error since we checked to make sure the value
        // is an integer earlier
        Data::Integer(i) => *this.as_mut() = i.to_bigint().unwrap(),
    }

    Ok(())
}

fn get_class_handle<'wren, 'a>(
    context: &mut wren::Context<'wren, NoTypeInfo, Foreign>,
    user_data: &'a mut MyUserData<'wren>,
) -> Option<&'a Handle<'wren>> {
    if user_data.big_int_data.is_none() {
        let class_handle = context.get_variable("bigint", "BigInt", 0)?;
        user_data.big_int_data = Some(BigIntModule(class_handle));
    };
    let data = user_data.big_int_data.as_ref().unwrap();
    Some(&data.0)
}
