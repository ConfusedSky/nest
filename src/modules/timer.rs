#![allow(unsafe_code)]

use tokio::time::{sleep, Duration};

use crate::wren;
use crate::MyUserData;

use super::scheduler::get;

pub unsafe fn start(vm: wren::VMPtr) {
    let scheduler = get().unwrap();
    let user_data = vm.get_user_data::<MyUserData>().unwrap();

    // We are guarenteed ms is positive based on usage
    let ms = vm.get_slot_double_unchecked(1) as u32;
    let fiber = vm.get_slot_handle_unchecked(2);

    user_data.schedule_task(async move {
        sleep(Duration::from_millis(ms.into())).await;
        scheduler.resume(fiber, false);
    });
}
