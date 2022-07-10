//! This is the simplest happy path

use wren::context::Native;
use wren::test::Context;
use wren_macros::foreign_static_method;

#[foreign_static_method]
fn foreign_test(context: &mut Context<'_, Native>, a: f64) -> f64 {
    assert!(context.get_user_data_mut().get_output().is_empty());

    a
}

fn main() {}
