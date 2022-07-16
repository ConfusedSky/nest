use tokio::time::{sleep, Duration};
use wren_macros::foreign_static_method;

use crate::Context;
use wren;
use wren::Handle;

use super::{source_file, Class, Module};

pub fn init_module<'wren>() -> Module<'wren> {
    let mut timer_class = Class::new();
    timer_class
        .static_methods
        .insert("startTimer_(_,_)", foreign_start);

    let mut timer_module = Module::new(source_file!("timer.wren"));
    timer_module.classes.insert("Timer", timer_class);

    timer_module
}

#[foreign_static_method]
fn start<'wren>(
    context: &mut Context<'wren>,
    ms: f64,
    handle: Handle<'wren>,
) -> Result<(), &'static str> {
    let scheduler = context
        .get_user_data_mut()
        .scheduler
        .as_mut()
        .ok_or("Scheduler not initialized")?;

    scheduler.schedule_task(
        async move { sleep(Duration::from_secs_f64(ms / 1000.0)).await },
        |context| {
            let fiber = context
                .check_fiber(handle)
                .expect("Fiber passed to Timer.start_(_,_) was not a valid fiber");
            fiber
                .transfer::<()>(context)
                .expect("Resume failed in Timer.start_(_,_)");
        },
    );
    Ok(())
}
