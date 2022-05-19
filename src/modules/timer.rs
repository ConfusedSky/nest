#![allow(unsafe_code)]

use tokio::time::{sleep, Duration};

use crate::wren;
use crate::MyUserData;

use super::scheduler::get;

pub unsafe fn start_timer(vm: wren::VMPtr) {
    let scheduler = get().unwrap();
    let user_data = vm.get_user_data::<MyUserData>().unwrap();

    let ms = vm.get_slot_double_unchecked(1) as u64;
    let fiber = vm.get_slot_handle_unchecked(2);

    let future = async move {
        sleep(Duration::from_millis(ms)).await;
        scheduler.resume(fiber, false);
    };

    user_data.enqueue_future(future);
}
