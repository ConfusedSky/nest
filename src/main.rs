#![deny(clippy::all)]
#![warn(clippy::pedantic, clippy::nursery)]
// Eventaully we will ban unsafe code, but we are still figuring out the abstractions
// right now
// #![warn(unsafe_code)]

#[macro_use]
extern crate lazy_static;

use std::{env, ffi::CString, fs, future::Future, path::PathBuf, pin::Pin, ptr::NonNull};
use tokio::runtime::Builder;

use wren::VMPtr;

mod modules;
mod wren;
mod wren_sys;

struct MyUserData {
    queue: Vec<Pin<Box<dyn Future<Output = ()>>>>,
    pub has_waiting_fibers: bool,
}

impl MyUserData {
    pub fn new() -> Self {
        Self {
            queue: Vec::default(),
            has_waiting_fibers: false,
        }
    }

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
}

impl wren::VmUserData for MyUserData {
    fn on_error(&mut self, _: VMPtr, kind: wren::ErrorKind) {
        match kind {
            wren::ErrorKind::Compile(ctx) => {
                println!("[{} line {}] [Error] {}", ctx.module, ctx.line, ctx.msg);
            }
            wren::ErrorKind::Runtime(msg) => println!("[Runtime Error] {}", msg),
            wren::ErrorKind::Stacktrace(ctx) => {
                println!("[{} line {}] in {}", ctx.module, ctx.line, ctx.msg);
            }
            wren::ErrorKind::Unknown(kind, ctx) => {
                println!(
                    "[{} line {}] [Unkown Error {}] {}",
                    ctx.module, ctx.line, kind, ctx.msg
                );
            }
        }
    }
    fn on_write(&mut self, _: VMPtr, text: &str) {
        print!("{}", text);
    }
    fn load_module(&mut self, name: &str) -> Option<&'static CString> {
        crate::modules::get_module(name).map(|module| &module.source)
    }
    fn bind_foreign_method(
        &mut self,
        module: &str,
        class_name: &str,
        is_static: bool,
        signature: &str,
    ) -> Option<wren::ForeignMethod> {
        let module = crate::modules::get_module(module)?;
        let class = module.classes.get(class_name)?;
        if is_static {
            class.static_methods.get(signature).copied()
        } else {
            class.methods.get(signature).copied()
        }
    }
}

fn main() {
    // There is always the executables name which we can skip
    let module: Option<String> = env::args().nth(1);

    if module.is_none() {
        println!("Please pass in the name of a script file to get started");
        return;
    }

    let module = module.unwrap();
    let mut module_path = PathBuf::new();
    module_path.push("scripts");
    module_path.push(&module);
    module_path.set_extension("wren");

    let source = fs::read_to_string(&module_path)
        .unwrap_or_else(|_| panic!("Ensure {} is a valid module name to continue", &module));

    let user_data = MyUserData::new();
    let vm = wren::Vm::new(user_data).unwrap();

    let result = vm.interpret(module, source);

    match result {
        Ok(()) => (),
        Err(wren::InterpretResultErrorKind::Compile) => panic!("COMPILE_ERROR"),
        Err(wren::InterpretResultErrorKind::Runtime) => panic!("RUNTIME_ERROR"),
        Err(wren::InterpretResultErrorKind::Unknown(kind)) => panic!("UNKNOWN ERROR: {}", kind),
    }

    let runtime = Builder::new_current_thread().enable_all().build().unwrap();

    // SAFETY: If userdata still exists it's going to be the same type that we passed in
    #[allow(unsafe_code)]
    let user_data = unsafe { vm.get_ptr().get_user_data::<MyUserData>() };
    if let Some(user_data) = user_data {
        loop {
            user_data.run_async_loop(&runtime);
            if user_data.has_waiting_fibers {
                unsafe {
                    modules::scheduler::get().unwrap().resume_waiting();
                }
            } else {
                break;
            }
        }
    }
}
