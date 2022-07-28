#![allow(unsafe_code)]

use std::ops::Add;

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
    bigint_class.methods.insert("-(_)", sub);
    bigint_class.methods.insert("*(_)", mul);
    bigint_class.static_methods.insert("fib(_)", fib);
    bigint_class.static_methods.insert("fastfib(_)", fast_fib);

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
            Ok(Data::BigInt(i)) => (&*this).add(&*i),
            Ok(Data::Integer(i)) => (&*this).add(i),
            Err(s) => {
                context.abort_fiber(s);
                return;
            }
        }
    };

    send_new_foreign(&mut context, result);
}

fn sub(mut context: Context) {
    let this = unsafe { ForeignClass::<BigInt>::get_slot(&mut context, 0) };
    let result = {
        match get_data(&mut context, "-(_)") {
            Ok(Data::BigInt(ref i)) => this.as_ref() - i.as_ref(),
            Ok(Data::Integer(i)) => this.as_ref() - i,
            Err(s) => {
                context.abort_fiber(s);
                return;
            }
        }
    };

    send_new_foreign(&mut context, result);
}

fn mul(mut context: Context) {
    let this = unsafe { ForeignClass::<BigInt>::get_slot(&mut context, 0) };
    let result = {
        match get_data(&mut context, "*(_)") {
            Ok(Data::BigInt(ref i)) => this.as_ref() * i.as_ref(),
            Ok(Data::Integer(i)) => this.as_ref() * i,
            Err(s) => {
                context.abort_fiber(s);
                return;
            }
        }
    };

    send_new_foreign(&mut context, result);
}

#[foreign_method]
fn to_string(this: ForeignClass<BigInt>) -> String {
    this.to_string()
}

fn fib(mut context: Context) {
    let (_, n) = context.try_get_stack::<((), f64)>();
    let n = match n {
        #[allow(clippy::cast_possible_truncation)]
        #[allow(clippy::cast_sign_loss)]
        Ok(n) if (n.trunc() - n).abs() < f64::EPSILON && n > 0.0 => n as usize,
        _ => {
            context.abort_fiber("BigInt.fib needs to be passed a positive integer");
            return;
        }
    };

    let mut f0: BigInt = 0u64.to_bigint().unwrap();
    let mut f1: BigInt = 1u64.to_bigint().unwrap();

    for _ in 0..n {
        let f2 = f0 + &f1;
        f0 = std::mem::replace(&mut f1, f2);
    }

    send_new_foreign(&mut context, f0);
}

fn fast_fib(mut context: Context) {
    #[allow(clippy::many_single_char_names)]
    fn helper(n: usize) -> (BigInt, BigInt) {
        if n == 0 {
            let zero: BigInt = 0u64.to_bigint().unwrap();
            let one: BigInt = 1u64.to_bigint().unwrap();
            (zero, one)
        } else {
            let (a, b) = helper(n / 2);
            let c = &a * (&b * 2 - &a);
            let d = &a * &a + &b * &b;

            if n % 2 == 0 {
                (c, d)
            } else {
                let e = c + &d;
                (d, e)
            }
        }
    }

    let (_, n) = context.try_get_stack::<((), f64)>();
    let n = match n {
        #[allow(clippy::cast_possible_truncation)]
        #[allow(clippy::cast_sign_loss)]
        Ok(n) if (n.trunc() - n).abs() < f64::EPSILON && n > 0.0 => n as usize,
        _ => {
            context.abort_fiber("BigInt.fastfib needs to be passed a positive integer");
            return;
        }
    };

    let result = helper(n).0;

    send_new_foreign(&mut context, result);
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

fn send_new_foreign(context: &mut Context, data: BigInt) {
    let (user_data, context) = context.get_user_data_mut_with_context();
    let class_handle = get_class_handle(context, user_data);
    match &class_handle {
        Some(handle) => unsafe {
            context.create_new_foreign(handle, data);
        },
        None => context.abort_fiber("Could not load the BigInt class"),
    }
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
