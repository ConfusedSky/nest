#![allow(unsafe_code)]

use tokio::time::{sleep, Duration};

use crate::wren;
use crate::MyUserData;

pub unsafe fn start(vm: wren::VMPtr) {
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
