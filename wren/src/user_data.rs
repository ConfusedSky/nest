use std::ffi::{CStr, CString};

use super::{context, Context, ErrorKind, ForeignMethod};

#[allow(unused_variables)]
// We define empty defaults here so that the user can define what they want
pub trait UserData<'wren, T> {
    fn resolve_module(&mut self, resolver: &str, name: &str) -> Option<CString> {
        CString::new(name).ok()
    }
    fn load_module(&mut self, name: &str) -> Option<&CStr> {
        None
    }
    fn bind_foreign_method(
        &mut self,
        module: &str,
        classname: &str,
        is_static: bool,
        signature: &str,
    ) -> Option<ForeignMethod<'wren, T>> {
        unsafe { std::mem::zeroed() }
    }
    // Default behavior is to return a struct with fields nulled out
    // so this is fine
    fn bind_foreign_class(
        &mut self,
        module: &str,
        classname: &str,
    ) -> wren_sys::WrenForeignClassMethods {
        unsafe { std::mem::zeroed() }
    }
    fn on_write(&mut self, vm: Context<'wren, T, context::Foreign>, text: &str) {}
    fn on_error(&mut self, vm: Context<'wren, T, context::Foreign>, kind: ErrorKind) {}
}

pub fn on_error(kind: super::ErrorKind) {
    match kind {
        super::ErrorKind::Compile(ctx) => {
            println!("[{} line {}] [Error] {}", ctx.module, ctx.line, ctx.msg);
        }
        super::ErrorKind::Runtime(msg) => println!("[Runtime Error] {}", msg),
        super::ErrorKind::Stacktrace(ctx) => {
            println!("[{} line {}] in {}", ctx.module, ctx.line, ctx.msg);
        }
        super::ErrorKind::Unknown(kind, ctx) => {
            println!(
                "[{} line {}] [Unkown Error {}] {}",
                ctx.module, ctx.line, kind, ctx.msg
            );
        }
    }
}
