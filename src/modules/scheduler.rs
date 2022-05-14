#![allow(unsafe_code)]

use std::ptr::NonNull;

use crate::{wren, wren_sys};

#[derive(Debug)]
struct Scheduler {
    vm: wren::VMPtr,
    // A handle to the "Scheduler" class object. Used to call static methods on it.
    class: NonNull<wren_sys::WrenHandle>,

    // This method resumes a fiber that is suspended waiting on an asynchronous
    // operation. The first resumes it with zero arguments, and the second passes
    // one.
    resume1: NonNull<wren_sys::WrenHandle>,
    resume2: NonNull<wren_sys::WrenHandle>,
    resume_error: NonNull<wren_sys::WrenHandle>,
}

impl Scheduler {
    fn resume(&mut self) {}
}

unsafe impl Send for Scheduler {}
unsafe impl Sync for Scheduler {}

static mut SCHEDULER: Option<Scheduler> = None;

unsafe fn _resume(vm: wren::VMPtr, method: NonNull<wren_sys::WrenHandle>) {
    let result = vm.call(method);

    if let Err(wren::InterpretResultErrorKind::Runtime) = result {
        panic!("AHHHHHHHHH")
    }
}

pub unsafe fn capture_methods(vm: wren::VMPtr) {
    vm.ensure_slots(1);
    vm.get_variable_unchecked("scheduler", "Scheduler", 0);
    // TODO: Figure out if we actually should check this
    let class = vm.get_slot_handle_unchecked(0);

    let resume1 = vm.make_call_handle("resume_(_)");
    let resume2 = vm.make_call_handle("resume_(_,_)");
    let resume_error = vm.make_call_handle("resumeError_(_,_)");

    SCHEDULER = Some(Scheduler {
        vm,
        class,
        resume1,
        resume2,
        resume_error,
    });
}
