use wren::ForeignClassMethods;
use wren_macros::foreign_method;

use crate::source_file;

use super::{Class, Module};
use wren::ForeignClass;

pub fn init_module<'wren>() -> Module<'wren> {
    let mut bigint_class = Class::new();
    bigint_class.foreign_class_methods = Some(ForeignClassMethods::new::<BigInt>());
    bigint_class.methods.insert("data", foreign_get_data);
    bigint_class.methods.insert("data=(_)", foreign_set_data);

    let mut module = Module::new(source_file!("bigint.wren"));
    module.classes.insert("BigInt", bigint_class);

    module
}

#[derive(Default)]
struct BigInt {
    marker_data: u8,
}

#[foreign_method]
#[allow(clippy::needless_pass_by_value)]
fn get_data(this: ForeignClass<BigInt>) -> f64 {
    this.marker_data.into()
}

#[allow(clippy::only_used_in_recursion)]
#[foreign_method]
fn set_data(mut this: ForeignClass<BigInt>, new_data: f64) -> Result<f64, &'static str> {
    if !(0.0_f64..=255.0_f64).contains(&new_data) {
        return Err("Data must be between 0 and 255!");
    }
    #[allow(clippy::cast_sign_loss)]
    #[allow(clippy::cast_possible_truncation)]
    {
        this.marker_data = new_data as u8;
    }
    Ok(new_data)
}
