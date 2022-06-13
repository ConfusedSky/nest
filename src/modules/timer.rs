#![allow(unsafe_code)]

use tokio::time::{sleep, Duration};

use crate::wren;
use crate::MyUserData;

use super::{Class, Module};
use std::ffi::CString;

pub fn init_module() -> Module {
    let timer_source = include_str!("timer.wren");

    let mut timer_class = Class::new();
    timer_class
        .static_methods
        .insert("startTimer_(_,_)".to_string(), start);

    let mut timer_module = Module::new(CString::new(timer_source).unwrap());
    timer_module
        .classes
        .insert("Timer".to_string(), timer_class);

    timer_module
}

unsafe fn start(vm: wren::VMPtr) {
    let user_data = vm.get_user_data::<MyUserData>().unwrap();
    let scheduler = user_data.scheduler.as_mut().unwrap();

    // We are guarenteed ms is positive based on usage
    let ms = vm.get_slot_double_unchecked(1) as u32;
    let fiber = vm.get_slot_handle_unchecked(2);

    let task = async move {
        sleep(Duration::from_millis(ms.into())).await;
        let user_data = vm.get_user_data::<MyUserData>().unwrap();
        let scheduler = user_data.scheduler.as_ref().unwrap();
        scheduler.resume(fiber, false);
    };

    scheduler.schedule_task(task);
}
