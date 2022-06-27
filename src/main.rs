#![deny(clippy::all)]
#![warn(clippy::pedantic, clippy::nursery)]
// Eventaully we will ban unsafe code, but we are still figuring out the abstractions
// right now
// #![warn(unsafe_code)]

use modules::{scheduler::Scheduler, Modules};
use std::{env, ffi::CStr, fs, path::PathBuf};
use tokio::runtime::Builder;

use wren::VmContext;

type Context<'wren> = VmContext<'wren, MyUserData<'wren>>;
type Handle<'wren> = crate::wren::Handle<'wren>;
type ForeignMethod<'wren> = crate::wren::ForeignMethod<'wren, MyUserData<'wren>>;

macro_rules! create_trait_alias {
    ($name:ident, $($bounds:tt)*) => {
        pub trait $name<'wren>: $($bounds)* {}
        impl <'wren, T: $($bounds)* > $name<'wren> for T {}
    };
}

create_trait_alias!(WrenGet, crate::wren::Get<'wren>);
create_trait_alias!(WrenSet, crate::wren::Set<'wren>);
create_trait_alias!(WrenGetArgs, crate::wren::GetArgs<'wren>);
create_trait_alias!(WrenSetArgs, crate::wren::SetArgs<'wren>);

mod modules;
mod wren;

pub struct MyUserData<'wren> {
    scheduler: Option<Scheduler<'wren>>,
    modules: Modules<'wren>,
}

impl<'wren> Default for MyUserData<'wren> {
    fn default() -> Self {
        Self {
            scheduler: None,
            modules: Modules::new(),
        }
    }
}

impl<'wren> wren::VmUserData<'wren, MyUserData<'wren>> for MyUserData<'wren> {
    fn on_error(&mut self, _: Context<'wren>, kind: wren::ErrorKind) {
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
    fn on_write(&mut self, _: Context<'wren>, text: &str) {
        print!("{}", text);
    }
    fn load_module(&mut self, name: &str) -> Option<&'wren CStr> {
        self.modules
            .get_module(name)
            .map(|module| &module.source)
            .copied()
    }
    fn bind_foreign_method(
        &mut self,
        module: &str,
        class_name: &str,
        is_static: bool,
        signature: &str,
    ) -> Option<wren::ForeignMethod<'wren, Self>> {
        let module = self.modules.get_module(module)?;
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

    let user_data = MyUserData::default();
    let mut vm_ = wren::Vm::new(user_data);
    let vm = vm_.get_context();

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
    let user_data = vm.get_user_data_mut();
    if let Some(user_data) = user_data {
        // We only should run the async loop if there is a loop to run
        if let Some(ref mut scheduler) = user_data.scheduler {
            loop {
                scheduler.run_async_loop(&runtime);

                // If there are waiting fibers or fibers that have been scheduled
                // but never had control handed over to them make sure they get a chance to run
                if scheduler.has_waiting_fibers {
                    unsafe {
                        scheduler.resume_waiting();
                    }
                } else if unsafe { scheduler.has_next() } {
                    unsafe { scheduler.run_next_scheduled() }
                } else {
                    break;
                }
            }
        }
    }

    // This code is for testing with leaks
    #[cfg(feature = "leaks")]
    {
        use std::io::stdin;
        drop(vm_);
        let mut buf = String::new();
        stdin().read_line(&mut buf).unwrap();
    }
}
