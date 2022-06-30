use std::borrow::Cow;
use std::mem::MaybeUninit;
use std::ptr::null;
use std::{ffi::CStr, mem::transmute_copy};

use ffi::WrenErrorType;
use wren_sys as ffi;
use wren_sys::WrenVM;

use super::{ErrorContext, ErrorKind, SystemUserData, VmContext, VmUserData};

pub(super) unsafe fn get_system_user_data<'s, V>(vm: *mut WrenVM) -> &'s mut SystemUserData<'s, V> {
    let user_data = ffi::wrenGetUserData(vm);
    if user_data.is_null() {
        panic!("User data should never be null!");
    } else {
        user_data.cast::<SystemUserData<V>>().as_mut().unwrap()
    }
}

unsafe extern "C" fn resolve_module<'wren, V: 'wren + VmUserData<'wren, V>>(
    vm: *mut WrenVM,
    resolver: *const i8,
    name: *const i8,
) -> *const i8 {
    let mut context: VmContext<V> = VmContext::new_unchecked(vm);
    let user_data = context.get_user_data_mut();

    let name = CStr::from_ptr(name).to_string_lossy();
    let resolver = CStr::from_ptr(resolver).to_string_lossy();

    let name = user_data.resolve_module(resolver.as_ref(), name.as_ref());

    match name {
        Some(name) => name.into_raw(),
        None => null(),
    }
}

unsafe extern "C" fn load_module<'wren, V: 'wren + VmUserData<'wren, V>>(
    vm: *mut WrenVM,
    name: *const i8,
) -> wren_sys::WrenLoadModuleResult {
    let mut context: VmContext<V> = VmContext::new_unchecked(vm);
    let user_data = context.get_user_data_mut();

    let name = CStr::from_ptr(name).to_string_lossy();
    let source = user_data.load_module(name.as_ref());
    let mut result: wren_sys::WrenLoadModuleResult = std::mem::zeroed();

    if let Some(source) = source {
        // SAFETY: we use into raw here and pass in a function that frees the memory
        result.source = source.as_ptr();
    }

    result
}

unsafe extern "C" fn bind_foreign_method<'wren, V: 'wren + VmUserData<'wren, V>>(
    vm: *mut WrenVM,
    module: *const i8,
    class_name: *const i8,
    is_static: bool,
    signature: *const i8,
) -> wren_sys::WrenForeignMethodFn {
    let mut context: VmContext<V> = VmContext::new_unchecked(vm);
    let user_data = context.get_user_data_mut();

    let module = CStr::from_ptr(module).to_string_lossy();
    let class_name = CStr::from_ptr(class_name).to_string_lossy();
    let signature = CStr::from_ptr(signature).to_string_lossy();

    let method = user_data.bind_foreign_method(
        module.as_ref(),
        class_name.as_ref(),
        is_static,
        signature.as_ref(),
    )?;

    // Safety: VMContext is a transparent wrapper over a *mut WrenVM
    transmute_copy(&method)
}

unsafe extern "C" fn write_fn<'wren, V: 'wren + VmUserData<'wren, V>>(
    vm: *mut WrenVM,
    text: *const i8,
) {
    let mut context: VmContext<V> = VmContext::new_unchecked(vm);
    let user_data = context.get_user_data_mut();

    let text = CStr::from_ptr(text).to_string_lossy();
    user_data.on_write(VmContext::new_unchecked(vm), text.as_ref());
}

unsafe extern "C" fn error_fn<'wren, V: 'wren + VmUserData<'wren, V>>(
    vm: *mut WrenVM,
    error_type: WrenErrorType,
    module: *const i8,
    line: i32,
    msg: *const i8,
) {
    let mut context: VmContext<V> = VmContext::new_unchecked(vm);
    let user_data = context.get_user_data_mut();

    let msg = CStr::from_ptr(msg).to_string_lossy();
    // This lives outside of the if statement so that it can live long enough
    // to be passed to user_data on error
    let c_module: Cow<str>;
    // Runtime doesn't have a valid module so it will crash if it goes any further
    let kind = if error_type == wren_sys::WrenErrorType_WREN_ERROR_RUNTIME {
        ErrorKind::Runtime(msg.as_ref())
    } else {
        c_module = CStr::from_ptr(module).to_string_lossy();
        let context = ErrorContext {
            module: c_module.as_ref(),
            line,
            msg: msg.as_ref(),
        };
        match error_type {
            wren_sys::WrenErrorType_WREN_ERROR_COMPILE => ErrorKind::Compile(context),
            wren_sys::WrenErrorType_WREN_ERROR_RUNTIME => ErrorKind::Runtime(msg.as_ref()),
            wren_sys::WrenErrorType_WREN_ERROR_STACK_TRACE => ErrorKind::Stacktrace(context),
            kind => ErrorKind::Unknown(kind, context),
        }
    };

    user_data.on_error(VmContext::new_unchecked(vm), kind);
}

pub(super) fn init_config<'wren, V>() -> ffi::WrenConfiguration
where
    V: 'wren + VmUserData<'wren, V>,
{
    let mut config: MaybeUninit<ffi::WrenConfiguration> = MaybeUninit::zeroed();
    let mut config = unsafe {
        ffi::wrenInitConfiguration(config.as_mut_ptr());
        config.assume_init()
    };

    config.writeFn = Some(write_fn::<V>);
    config.errorFn = Some(error_fn::<V>);
    config.loadModuleFn = Some(load_module::<V>);
    config.resolveModuleFn = Some(resolve_module::<V>);
    config.bindForeignMethodFn = Some(bind_foreign_method::<V>);

    config
}
