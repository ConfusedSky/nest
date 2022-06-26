#![allow(unsafe_code)]

use std::{future::Future, pin::Pin};

use crate::{wren, Context, Handle, WrenSet};

use super::{source_file, Class, Module};

unsafe fn _resume<'wren>(vm: &mut Context<'wren>, method: &Handle<'wren>) {
    let result = vm.call(method);

    if let Err(wren::InterpretResultErrorKind::Runtime) = result {
        panic!("Fiber panicked after resuming.");
    }
}

pub fn init_module<'wren>() -> Module<'wren> {
    let mut scheduler_class = Class::new();
    scheduler_class
        .static_methods
        .insert("captureMethods_()", capture_methods);
    scheduler_class
        .static_methods
        .insert("awaitAll_()", await_all);

    let mut scheduler_module = Module::new(source_file!("scheduler.wren"));
    scheduler_module
        .classes
        .insert("Scheduler", scheduler_class);

    scheduler_module
}

// #[derive(Debug)]
pub struct Scheduler<'wren> {
    vm: Context<'wren>,
    // A handle to the "Scheduler" class object. Used to call static methods on it.
    class: Handle<'wren>,

    // This method resumes a fiber that is suspended waiting on an asynchronous
    // operation. The first resumes it with zero arguments, and the second passes
    // one.
    resume1: Handle<'wren>,
    resume2: Handle<'wren>,
    resume_error: Handle<'wren>,
    resume_waiting: Handle<'wren>,
    has_next: Handle<'wren>,
    run_next_scheduled: Handle<'wren>,

    pub has_waiting_fibers: bool,
    queue: Vec<Pin<Box<dyn Future<Output = ()>>>>,
}

impl<'wren> Scheduler<'wren> {
    pub fn schedule_task<F>(&mut self, future: F)
    where
        F: 'static + Future<Output = ()>,
    {
        let future = Box::pin(future);
        self.queue.insert(0, future);
    }

    pub fn next_item(&mut self) -> Option<Pin<Box<dyn Future<Output = ()>>>> {
        self.queue.pop()
    }

    pub fn await_all(&mut self) {
        self.has_waiting_fibers = true;
    }

    /// Loop as long as new tasks are still being created
    /// Loop is structured this way so that mutiple items can be
    /// added to the queue from a single Fiber and multiple asynchronous calls
    /// can be made from a single fiber as well.
    /// If each call awaited imidiately this would still work but all tasks would complete in
    /// order they were enqueued, which would cause faster processes to wait for slower
    /// processes if they were scheduled after the slower process.
    ///
    /// For example if you had two Fibers with timers
    /// ```
    /// Scheduler.add {
    ///   Timer.sleep(1000)
    ///   System.print("Task 1 complete")
    /// }
    /// Scheduler.add {
    ///   Timer.sleep(500)
    ///   System.print("Task 2 complete")
    /// }
    /// Scheduler.awaitAll()
    /// ```
    ///
    /// Would result in "Task 1 complete" printing before "Task 2 complete" printing
    ///
    /// And if we only spawned the handles that exist at the time of calling without
    /// looping then each Fiber could only have one async call in it with any other
    /// call in that fiber not being awaited on. This is because new async calls
    /// are never spawned on the async runtime
    ///
    /// So
    /// ```
    /// Scheduler.add {
    ///   Timer.sleep(100)
    ///   System.print("Do 1")
    ///   Timer.sleep(100)
    ///   System.print("Do 2")
    /// }
    /// Scheduler.awaitAll()
    /// ```
    /// Would only print "Do 1"
    pub fn run_async_loop(&mut self, runtime: &tokio::runtime::Runtime) {
        let local_set = tokio::task::LocalSet::new();

        let mut handles = vec![];
        let mut next = self.next_item();

        runtime.block_on(local_set.run_until(async move {
            loop {
                // Create a new task on the local set for each of the scheduled tasks
                // So that they can be run concurrently
                while let Some(future) = next {
                    handles.push(tokio::task::spawn_local(future));
                    next = self.next_item();
                }

                // If there are no new handles then break out of the loop
                if handles.is_empty() {
                    break;
                }

                // Wait for existing handles then clear the handles
                for handle in &mut handles {
                    handle.await.unwrap();
                }
                handles.clear();

                // Check the queue for another handle
                next = self.next_item();
            }
        }));
    }

    pub unsafe fn resume(&mut self, fiber: Handle<'wren>) {
        // resume_wit_arg needs a valid WrenValue type so just set it to
        // a random one
        self.vm.set_stack(&(&self.class, &fiber));
        // this is just here to keep clippy from complaining
        drop(fiber);
        _resume(&mut self.vm, &self.resume1);
    }

    pub unsafe fn resume_with_arg<T: WrenSet<'wren>>(
        &mut self,
        fiber: Handle<'wren>,
        additional_argument: T,
    ) {
        self.vm
            .set_stack(&(&self.class, &fiber, &additional_argument));
        drop(fiber);
        _resume(&mut self.vm, &self.resume2);
    }
    pub unsafe fn resume_error<S>(&mut self, fiber: Handle<'wren>, error: S)
    where
        S: AsRef<str>,
    {
        let error = error.as_ref().to_string();
        self.vm.set_stack(&(&self.class, &fiber, &error));
        drop(fiber);
        _resume(&mut self.vm, &self.resume_error);
    }
    pub unsafe fn resume_waiting(&mut self) {
        self.has_waiting_fibers = false;
        self.vm.set_stack(&self.class);
        _resume(&mut self.vm, &self.resume_waiting);
    }
    pub unsafe fn has_next(&mut self) -> bool {
        self.vm.set_stack(&self.class);
        _resume(&mut self.vm, &self.has_next);

        self.vm.get_return_value()
    }
    pub unsafe fn run_next_scheduled(&mut self) {
        self.vm.set_stack(&self.class);
        _resume(&mut self.vm, &self.run_next_scheduled);
    }
}

unsafe fn capture_methods<'wren>(mut vm: Context<'wren>) {
    let mut user_data = vm.get_user_data().unwrap();
    vm.ensure_slots(1);
    let class = vm.get_variable_unchecked("scheduler", "Scheduler", 0);

    let resume1 = wren::make_call_handle!(vm, "resume_(_)");
    let resume2 = wren::make_call_handle!(vm, "resume_(_,_)");
    let resume_error = wren::make_call_handle!(vm, "resumeError_(_,_)");
    let resume_waiting = wren::make_call_handle!(vm, "resumeWaitingFibers_()");
    let has_next = wren::make_call_handle!(vm, "hasNext_");
    let run_next_scheduled = wren::make_call_handle!(vm, "runNextScheduled_()");

    let scheduler: Option<Scheduler<'wren>> = Some(Scheduler {
        queue: Vec::default(),
        has_waiting_fibers: false,
        vm,
        class,
        resume1,
        resume2,
        resume_error,
        resume_waiting,
        has_next,
        run_next_scheduled,
    });

    user_data.scheduler = scheduler;
}

#[allow(clippy::needless_pass_by_value)]
unsafe fn await_all(vm: Context) {
    vm.get_user_data()
        .unwrap()
        .scheduler
        .as_mut()
        .unwrap()
        .await_all();
}
