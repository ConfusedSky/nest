#![allow(unsafe_code)]

use std::{cell::RefCell, ptr::NonNull};

use crate::{wren, wren_sys};

static mut SCHEDULER: RefCell<Option<Scheduler>> = RefCell::new(None);

unsafe fn _resume(vm: wren::VMPtr, method: NonNull<wren_sys::WrenHandle>) {
    let result = vm.call(method);

    if let Err(wren::InterpretResultErrorKind::Runtime) = result {
        panic!("AHHHHHHHHH")
    }
}
#[derive(Debug)]
pub struct Scheduler {
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

impl Drop for Scheduler {
    fn drop(&mut self) {
        let scheduler = self;
        let vm = scheduler.vm;
        unsafe {
            vm.release_handle_unchecked(scheduler.class);
            vm.release_handle_unchecked(scheduler.resume1);
            vm.release_handle_unchecked(scheduler.resume2);
            vm.release_handle_unchecked(scheduler.resume_error);
        }
    }
}

impl Scheduler {
    pub unsafe fn resume(&self, fiber: NonNull<wren_sys::WrenHandle>, has_argument: bool) {
        self.vm.ensure_slots(2 + if has_argument { 1 } else { 0 });
        self.vm.set_slot_handle_unchecked(0, self.class);
        self.vm.set_slot_handle_unchecked(1, fiber);
        self.vm.release_handle_unchecked(fiber);

        if !has_argument {
            _resume(self.vm, self.resume1);
        }
    }
    pub unsafe fn finish_resume(&self) {
        _resume(self.vm, self.resume2);
    }
    pub unsafe fn resume_error<S>(&self, fiber: NonNull<wren_sys::WrenHandle>, error: S)
    where
        S: AsRef<str>,
    {
        self.resume(fiber, true);
        self.vm.set_slot_string_unchecked(2, error);
        _resume(self.vm, self.resume_error);
    }
}

unsafe impl Send for Scheduler {}
unsafe impl Sync for Scheduler {}

pub unsafe fn capture_methods(vm: wren::VMPtr) {
    vm.ensure_slots(1);
    vm.get_variable_unchecked("scheduler", "Scheduler", 0);
    // TODO: Figure out if we actually should check this
    let class = vm.get_slot_handle_unchecked(0);

    let resume1 = vm.make_call_handle("resume_(_)");
    let resume2 = vm.make_call_handle("resume_(_,_)");
    let resume_error = vm.make_call_handle("resumeError_(_,_)");

    let scheduler = SCHEDULER.get_mut();
    *scheduler = Some(Scheduler {
        vm,
        class,
        resume1,
        resume2,
        resume_error,
    });
}

pub unsafe fn get<'s>() -> Option<&'s Scheduler> {
    SCHEDULER.get_mut().as_ref()
}
