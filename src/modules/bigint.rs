#![allow(unsafe_code)]

use num_bigint::{BigInt, ToBigInt};
use num_traits::{Num, One, Zero};
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
    bigint_class.methods.insert("/(_)", div);
    bigint_class.methods.insert("pow(_)", pow);
    bigint_class.static_methods.insert("new(_)", new);
    bigint_class.static_methods.insert("fib(_)", fib);
    bigint_class.static_methods.insert("fastfib(_)", fast_fib);
    bigint_class.static_methods.insert("ZERO", zero);
    bigint_class.static_methods.insert("ONE", one);

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
    *this = internal_set_value(context, "setValue")?;
    Ok(())
}

fn implement_operator<'wren, B, I>(
    context: &mut Context<'wren>,
    method: &'static str,
    big_int_method: &B,
    integer_method: &I,
) where
    B: Fn(&BigInt, &BigInt) -> BigInt,
    I: Fn(&BigInt, i64) -> BigInt,
{
    let this = unsafe { ForeignClass::<'wren, BigInt>::get_slot(context, 0) };
    let result = {
        match get_data(context, method, false) {
            Ok(Data::BigInt(i)) => big_int_method(&*this, &*i),
            Ok(Data::Integer(i)) => integer_method(&*this, i),
            Err(s) => {
                context.abort_fiber(s);
                return;
            }
        }
    };

    send_new_foreign(context, result);
}

fn add(mut context: Context<'_>) {
    implement_operator(&mut context, "+(_)", &|a, b| a + b, &|a, b| a + b);
}

fn sub(mut context: Context<'_>) {
    implement_operator(&mut context, "-(_)", &|a, b| a - b, &|a, b| a - b);
}

fn mul(mut context: Context<'_>) {
    implement_operator(&mut context, "*(_)", &|a, b| a * b, &|a, b| a * b);
}

fn div(mut context: Context<'_>) {
    implement_operator(&mut context, "*(_)", &|a, b| a / b, &|a, b| a / b);
}

fn pow<'wren>(mut context: Context<'wren>) {
    let (this, n) = context.try_get_stack::<(ForeignClass<'wren, BigInt>, f64)>();
    let n = match n {
        #[allow(clippy::cast_possible_truncation)]
        #[allow(clippy::cast_sign_loss)]
        #[allow(clippy::cast_lossless)]
        Ok(n) if (n.trunc() - n).abs() < f64::EPSILON && n > 0.0 && n < u32::MAX as f64 => n as u32,
        _ => {
            context.abort_fiber(format!(
                "BigInt.pow(_) needs to be passed a positive integer with a max value of {}",
                u32::MAX
            ));
            return;
        }
    };

    let result = this.unwrap().pow(n);

    send_new_foreign(&mut context, result);
}

fn new(mut context: Context<'_>) {
    let this = internal_set_value(&mut context, "new(_)");
    match this {
        Ok(this) => send_new_foreign(&mut context, this),
        Err(e) => context.abort_fiber(e),
    }
}

fn zero(mut context: Context<'_>) {
    send_new_foreign(&mut context, Zero::zero());
}

fn one(mut context: Context<'_>) {
    send_new_foreign(&mut context, One::one());
}

#[foreign_method]
fn to_string(this: ForeignClass<'_, BigInt>) -> String {
    this.to_string()
}

fn fib(mut context: Context<'_>) {
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

    let mut f0: BigInt = Zero::zero();
    let mut f1: BigInt = One::one();

    for _ in 0..n {
        let f2 = f0 + &f1;
        f0 = std::mem::replace(&mut f1, f2);
    }

    send_new_foreign(&mut context, f0);
}

fn fast_fib(mut context: Context<'_>) {
    #[allow(clippy::many_single_char_names)]
    fn helper(n: usize) -> (BigInt, BigInt) {
        if n == 0 {
            let zero: BigInt = Zero::zero();
            let one: BigInt = One::one();
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

fn get_data<'wren>(
    context: &mut Context<'wren>,
    method: &str,
    accept_string: bool,
) -> Result<Data<'wren>, String> {
    // TODO: Implement better reflection so this error message is possible on the
    // rust side
    // "BigInt.%(method) expects a BigInt or an Integer got %(value): %(value.type)"
    let error = format!(
        "BigInt.{method} expects a BigInt{} or an Integer",
        if accept_string { ", String" } else { "" }
    );
    let slot_type = unsafe { context.get_slot_type(1) };
    match slot_type {
        WrenType::String if accept_string => {
            let slot = unsafe { String::get_slot_unchecked(context, 1, WrenType::String) };
            let bi = BigInt::from_str_radix(&slot, 10)
                .map_err(|_| format!("Failed to parse \"{slot}\" as an integer!"))?;
            // TODO: Do this better later
            send_new_foreign(context, bi);
            let slot = unsafe { ForeignClass::<BigInt>::try_get_slot_unchecked(context, 0) };
            let slot = slot.map_err(|_| error)?;
            Ok(Data::BigInt(slot))
        }
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

fn internal_set_value<'wren>(context: &mut Context<'wren>, method: &str) -> Result<BigInt, String> {
    let data = get_data(context, method, true)?;

    Ok(match data {
        Data::BigInt(i) => i.as_ref().clone(),
        // This shouldn't error since we checked to make sure the value
        // is an integer earlier
        Data::Integer(i) => i.to_bigint().unwrap(),
    })
}

fn send_new_foreign(context: &mut Context<'_>, data: BigInt) {
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
