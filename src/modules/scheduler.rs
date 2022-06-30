#![allow(unsafe_code)]

use std::{future::Future, pin::Pin};

use crate::{
    wren::{self, RawVMContext},
    Handle,
};

use super::{source_file, Class, Module};

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

type Context<'wren> = RawVMContext<'wren>;
type Task<'wren> = (
    Pin<Box<dyn 'static + Future<Output = ()>>>,
    Box<dyn 'wren + FnOnce(&mut Context<'wren>)>,
);

// #[derive(Debug)]
pub struct Scheduler<'wren> {
    // A handle to the "Scheduler" class object. Used to call static methods on it.
    class: Handle<'wren>,

    // This method resumes a fiber that is suspended waiting on an asynchronous
    // operation. The first resumes it with zero arguments, and the second passes
    // one.
    resume_waiting: Handle<'wren>,
    has_next: Handle<'wren>,
    run_next_scheduled: Handle<'wren>,

    pub has_waiting_fibers: bool,
    queue: Vec<Task<'wren>>,
}

impl<'wren> Scheduler<'wren> {
    pub fn schedule_task<F, P>(&mut self, future: F, post_task: P)
    where
        F: 'static + Future<Output = ()>,
        P: 'wren + FnOnce(&mut Context<'wren>),
    {
        let future = Box::pin(future);
        let post_task = Box::new(post_task);
        self.queue.push((future, post_task));
    }

    pub fn await_all(&mut self) {
        self.has_waiting_fibers = true;
    }

    //#region
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
    //#endregion
    pub fn run_async_loop(&mut self, vm: &mut Context<'wren>, runtime: &tokio::runtime::Runtime) {
        let local_set = tokio::task::LocalSet::new();
        let (tx, mut rx) = tokio::sync::mpsc::channel(128);

        let mut post_tasks = vec![];

        runtime.block_on(local_set.run_until(async move {
            loop {
                // Create a new task on the local set for each of the scheduled tasks
                // So that they can be run concurrently
                for (future, post_task) in self.queue.drain(..) {
                    let i = post_tasks.len();
                    let tx = tx.clone();
                    post_tasks.push(Some(post_task));
                    tokio::task::spawn_local(async move {
                        future.await;
                        tx.send(i).await.expect("Channel shoudn't fail to send");
                    });
                }

                // If there are no new handles then break out of the loop
                if post_tasks.is_empty() {
                    break;
                }

                // For each resume callback resume if
                // there is a resume handler
                // Wait for existing handlers then clear the handlers
                // Note that this clears the handlers in the order that they come in
                for _ in 0..post_tasks.len() {
                    tokio::select! {
                        Some(i) = rx.recv() => {
                            let post_task = &mut post_tasks[i];
                            post_task.take().unwrap()(vm);
                        }
                    }
                }
                post_tasks.clear();
            }
        }));
    }

    unsafe fn _resume(vm: &mut Context<'wren>, method: &Handle<'wren>) {
        let result = vm.call(method);

        if let Err(wren::InterpretResultErrorKind::Runtime) = result {
            panic!("Fiber errored after resuming.");
        }
    }
    pub unsafe fn resume_waiting(&mut self, vm: &mut Context<'wren>) {
        self.has_waiting_fibers = false;
        vm.set_stack(&self.class);
        Self::_resume(vm, &self.resume_waiting);
    }
    pub unsafe fn has_next(&mut self, vm: &mut Context<'wren>) -> bool {
        vm.set_stack(&self.class);
        Self::_resume(vm, &self.has_next);

        vm.get_return_value_unchecked()
    }
    pub unsafe fn run_next_scheduled(&mut self, vm: &mut Context<'wren>) {
        vm.set_stack(&self.class);
        Self::_resume(vm, &self.run_next_scheduled);
    }
}

fn capture_methods<'wren>(mut vm: crate::Context<'wren>) {
    use crate::wren::cstr;
    vm.ensure_slots(1);
    let class = vm
        .get_variable("scheduler", "Scheduler", 0)
        .expect("Scheduler variable hasn't been defined");

    let resume_waiting = vm.make_call_handle(cstr!("resumeWaitingFibers_()"));
    let has_next = vm.make_call_handle(cstr!("hasNext_"));
    let run_next_scheduled = vm.make_call_handle(cstr!("runNextScheduled_()"));

    let scheduler: Scheduler<'wren> = Scheduler {
        queue: Vec::default(),
        has_waiting_fibers: false,
        class,
        resume_waiting,
        has_next,
        run_next_scheduled,
    };

    vm.get_user_data_mut().scheduler = Some(scheduler);
}

#[allow(clippy::needless_pass_by_value)]
fn await_all(mut vm: crate::Context) {
    vm.get_user_data_mut()
        .scheduler
        .as_mut()
        .unwrap()
        .await_all();
}
