#![allow(unsafe_code)]

use tokio::time::{sleep, Duration};

use crate::wren;
use crate::Context;

use super::{source_file, Class, Module};
use crate::wren::Fiber;

pub fn init_module<'wren>() -> Module<'wren> {
    let mut timer_class = Class::new();
    timer_class.static_methods.insert("startTimer_(_,_)", start);

    let mut timer_module = Module::new(source_file!("timer.wren"));
    timer_module.classes.insert("Timer", timer_class);

    timer_module
}

fn start(mut vm: Context) {
    // SAFETY: We are guarenteed to have two arguments passed back,
    // ms is positive and Fiber is a genuine fiber because start isn't
    // in the public interface and these are cheked in the wren side
    let (_, ms, fiber) = unsafe { vm.get_stack_unchecked::<((), f64, Fiber)>() };

    let scheduler = vm.get_user_data_mut().scheduler.as_mut().unwrap();
    scheduler.schedule_task(
        async move {
            sleep(Duration::from_secs_f64(ms / 1000.0)).await;
        },
        |vm| fiber.transfer(vm).expect("Resume failed in timer start"),
    );
}
