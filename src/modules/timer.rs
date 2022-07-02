#![allow(unsafe_code)]

use tokio::time::{sleep, Duration};

use crate::wren;
use crate::wren::Handle;
use crate::Context;

use super::{source_file, Class, Module};

pub fn init_module<'wren>() -> Module<'wren> {
    let mut timer_class = Class::new();
    timer_class.static_methods.insert("startTimer_(_,_)", start);

    let mut timer_module = Module::new(source_file!("timer.wren"));
    timer_module.classes.insert("Timer", timer_class);

    timer_module
}

fn start(mut vm: Context) {
    // SAFETY: We are guarenteed to have two arguments passed back,
    // ms is positive and Fiber should always be Fiber current
    // because start isn't in the public interface, ms is checked on the wren side
    // and fiber is always passed as Fiber.current
    // Still we get a raw handle back here to make sure that the
    // fiber we create is genuine to prevent any UB
    let (_, ms, fiber) = unsafe { vm.get_stack_unchecked::<((), f64, Handle)>() };

    let scheduler = vm.get_user_data_mut().scheduler.as_mut().unwrap();
    scheduler.schedule_task(
        async move {
            sleep(Duration::from_secs_f64(ms / 1000.0)).await;
        },
        |vm| {
            let fiber = vm
                .check_fiber(fiber)
                .expect("Fiber passed to Timer.start_(_,_) was not a valid fiber");
            fiber
                .transfer::<()>(vm)
                .expect("Resume failed in Timer.start_(_,_)");
        },
    );
}
