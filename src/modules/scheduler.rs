#![allow(unsafe_code)]

use std::{cell::RefCell, ptr::NonNull};

use crate::{wren, MyUserData};
use wren_sys;

static mut SCHEDULER: RefCell<Option<Scheduler>> = RefCell::new(None);

unsafe fn _resume(vm: wren::VMPtr, method: NonNull<wren_sys::WrenHandle>) {
    let result = vm.call(method);

    if let Err(wren::InterpretResultErrorKind::Runtime) = result {
        panic!("Fiber panicked after resuming.");
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
    resume_waiting: NonNull<wren_sys::WrenHandle>,
    has_next: NonNull<wren_sys::WrenHandle>,
    run_next_scheduled: NonNull<wren_sys::WrenHandle>,
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
            vm.release_handle_unchecked(scheduler.resume_waiting);
            vm.release_handle_unchecked(scheduler.has_next);
            vm.release_handle_unchecked(scheduler.run_next_scheduled);
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
    pub unsafe fn resume_waiting(&self) {
        let mut user_data = self.vm.get_user_data::<MyUserData>().unwrap();
        user_data.has_waiting_fibers = false;
        self.vm.ensure_slots(1);
        self.vm.set_slot_handle_unchecked(0, self.class);
        _resume(self.vm, self.resume_waiting);
    }
    pub unsafe fn has_next(&self) -> bool {
        self.vm.ensure_slots(1);
        self.vm.set_slot_handle_unchecked(0, self.class);
        _resume(self.vm, self.has_next);
        self.vm.get_slot_bool_unchecked(0)
    }
    pub unsafe fn run_next_scheduled(&self) {
        self.vm.ensure_slots(1);
        self.vm.set_slot_handle_unchecked(0, self.class);
        _resume(self.vm, self.run_next_scheduled);
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
    let resume_waiting = vm.make_call_handle("resumeWaitingFibers_()");
    let has_next = vm.make_call_handle("hasNext_");
    let run_next_scheduled = vm.make_call_handle("runNextScheduled_()");

    let scheduler = SCHEDULER.get_mut();
    *scheduler = Some(Scheduler {
        vm,
        class,
        resume1,
        resume2,
        resume_error,
        resume_waiting,
        has_next,
        run_next_scheduled,
    });
}

pub unsafe fn await_all(vm: wren::VMPtr) {
    let mut user_data = vm.get_user_data::<MyUserData>().unwrap();
    user_data.has_waiting_fibers = true;
}

pub unsafe fn get<'s>() -> Option<&'s Scheduler> {
    SCHEDULER.get_mut().as_ref()
}
