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
