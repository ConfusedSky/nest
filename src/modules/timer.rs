#![allow(unsafe_code)]

use tokio::time::{sleep, Duration};

use crate::wren;
use crate::MyUserData;

use super::{Class, Module};
use crate::wren::Handle;
use std::ffi::CString;

pub fn init_module() -> Module {
    let mut timer_class = Class::new();
    timer_class.static_methods.insert("startTimer_(_,_)", start);

    let mut timer_module = Module::new(source_file!("timer.wren"));
    timer_module.classes.insert("Timer", timer_class);

    timer_module
}

unsafe fn start(vm: wren::VMPtr) {
    let user_data = vm.get_user_data::<MyUserData>().unwrap();
    let scheduler = user_data.scheduler.as_mut().unwrap();

    // We are guarenteed ms is positive based on usage
    let (_, ms, fiber) = vm.get_stack::<((), f64, Handle)>();

    let task = async move {
        sleep(Duration::from_secs_f64(ms / 1000.0)).await;
        let user_data = vm.get_user_data::<MyUserData>().unwrap();
        let scheduler = user_data.scheduler.as_ref().unwrap();
        scheduler.resume(fiber);
    };

    scheduler.schedule_task(task);
}
