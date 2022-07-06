#![deny(clippy::all)]
#![warn(clippy::pedantic, clippy::nursery)]
#![warn(unsafe_code)]

mod modules;
mod wren;
use modules::{scheduler::Scheduler, Modules};
use std::{env, ffi::CStr, fs, path::PathBuf};
use wren::context::{self, Location};

type Context<'wren> = wren::Context<'wren, MyUserData<'wren>, context::Foreign>;
type Handle<'wren> = crate::wren::Handle<'wren>;
type ForeignMethod<'wren> = crate::wren::ForeignMethod<'wren, MyUserData<'wren>>;

macro_rules! create_trait_alias {
    ($name:ident, $($bounds:tt)*) => {
        pub trait $name: $($bounds)* {}
        impl <T: $($bounds)* > $name for T {}
    };
    ($name:ident<$( $gen:tt $(: $bound:tt),* ),+>, $($bounds:tt)*) => {
        pub trait $name<$($gen $(: $bound),*),+>: $($bounds)* {}
        impl <$($gen $(: $bound),*),+, T: $($bounds)*> $name<$($gen),+> for T {}
    };
}

create_trait_alias!(WrenGet<'wren, L: Location>, crate::wren::GetValue<'wren, L>);
create_trait_alias!(WrenSet<'wren, L: Location>, crate::wren::SetValue<'wren, L>);
create_trait_alias!(
    WrenGetArgs<'wren, L: Location>,
    crate::wren::GetArgs<'wren, L>
);
create_trait_alias!(
    WrenSetArgs<'wren, L: Location>,
    crate::wren::SetArgs<'wren, L>
);

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
        wren::user_data::on_error(kind);
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
    let mut module_path = PathBuf::from(&module);
    if module_path.extension().is_none() {
        module_path.set_extension("wren");
    }

    let source = fs::read_to_string(&module_path)
        .unwrap_or_else(|_| panic!("Ensure {} is a valid module name to continue", &module));

    let user_data = MyUserData::default();
    let mut vm = wren::Vm::new(user_data);
    let context = vm.get_context();

    let result = context.interpret(module, source);

    match result {
        Ok(()) => (),
        Err(wren::InterpretResultErrorKind::Compile) => panic!("COMPILE_ERROR"),
        Err(wren::InterpretResultErrorKind::Runtime) => panic!("RUNTIME_ERROR"),
        Err(wren::InterpretResultErrorKind::IncorrectNumberOfArgsPassed) => {
            panic!("INCORRECT_NUMBER_OF_ARGS_PASSED")
        }
        Err(wren::InterpretResultErrorKind::Unknown(kind)) => panic!("UNKNOWN ERROR: {}", kind),
    }

    let (user_data, raw_context) = context.get_user_data_mut_with_context();
    // We only should run the async loop if there is a loop to run
    if let Some(ref mut scheduler) = user_data.scheduler {
        scheduler.run_async_loop(raw_context);
    }

    // This code is for testing with leaks
    #[cfg(feature = "leaks")]
    {
        use std::io::stdin;
        drop(vm);
        let mut buf = String::new();
        stdin().read_line(&mut buf).unwrap();
    }
}
