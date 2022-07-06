use criterion::{black_box, criterion_group, criterion_main, Criterion};
use wren::{context::Native, test::create_test_vm, CallHandle, GetValue, Handle};
use wren_sys as ffi;

fn raw_ffi<'wren, G: GetValue<'wren, Native>>(
    context: *mut ffi::WrenVM,
    test: *mut ffi::WrenHandle,
    add_three: *mut ffi::WrenHandle,
) {
    unsafe {
        ffi::wrenEnsureSlots(context, 4);

        ffi::wrenSetSlotHandle(context, 0, test);
        ffi::wrenSetSlotDouble(context, 1, black_box(1.0_f64));
        ffi::wrenSetSlotDouble(context, 2, black_box(2.0_f64));
        ffi::wrenSetSlotDouble(context, 3, black_box(3.0_f64));

        let result = ffi::wrenCall(context, add_three);

        match result {
            ffi::WrenInterpretResult_WREN_RESULT_SUCCESS => (),
            _ => panic!("Interpret failed"),
        }

        if std::mem::size_of::<G>() != 0 {
            let mut len = std::mem::MaybeUninit::uninit();
            let res = ffi::wrenGetSlotBytes(context, 0, len.as_mut_ptr()).cast();
            let len = len.assume_init().try_into().unwrap();
            let slice = std::slice::from_raw_parts(res, len);
            let str = String::from_utf8_lossy(slice).to_string();
        }
    }
}

fn unchecked<'wren, G: GetValue<'wren, Native>>(
    context: &mut wren::test::Context<'wren, Native>,
    test: &Handle<'wren>,
    add_three: &CallHandle<'wren>,
) {
    unsafe {
        context
            .call_unchecked::<G, _>(
                test,
                add_three,
                &(
                    &black_box(1.0_f64),
                    &black_box(2.0_f64),
                    &black_box(3.0_f64),
                ),
            )
            .unwrap();
    }
}

fn checked<'wren, G: GetValue<'wren, Native>>(
    context: &mut wren::test::Context<'wren, Native>,
    test: &Handle<'wren>,
    add_three: &CallHandle<'wren>,
) {
    context
        .call::<G, _>(
            test,
            add_three,
            &(
                &black_box(1.0_f64),
                &black_box(2.0_f64),
                &black_box(3.0_f64),
            ),
        )
        .unwrap();
}

pub fn call(c: &mut Criterion) {
    let (mut vm, test) = create_test_vm(
        "class Test {
            static add_three(a, b, c) { (a+b+c).toString }
        }",
        |_| {},
    );
    let context = vm.get_context();
    let add_three = context.make_call_handle(wren::cstr!("add_three(_,_,_)"));

    let context_ptr = context.as_ptr();
    let test_ptr = test.as_ptr();
    let add_three_ptr = add_three.as_ptr();

    let mut group = c.benchmark_group("Call");
    group.bench_function("Raw FFI", |b| {
        b.iter(|| raw_ffi::<String>(context_ptr, test_ptr, add_three_ptr));
    });

    group.bench_function("Unchecked", |b| {
        b.iter(|| unchecked::<String>(context, &test, &add_three))
    });

    group.bench_function("Checked", |b| {
        b.iter(|| checked::<String>(context, &test, &add_three))
    });
}

pub fn call_drop_output(c: &mut Criterion) {
    let (mut vm, test) = create_test_vm(
        "class Test {
            static add_three(a, b, c) { (a+b+c).toString }
        }",
        |_| {},
    );
    let context = vm.get_context();
    let add_three = context.make_call_handle(wren::cstr!("add_three(_,_,_)"));

    let context_ptr = context.as_ptr();
    let test_ptr = test.as_ptr();
    let add_three_ptr = add_three.as_ptr();

    let mut group = c.benchmark_group("Call Drop Output");
    group.bench_function("Raw FFI", |b| {
        b.iter(|| raw_ffi::<()>(context_ptr, test_ptr, add_three_ptr));
    });

    group.bench_function("Unchecked", |b| {
        b.iter(|| unchecked::<()>(context, &test, &add_three))
    });

    group.bench_function("Checked", |b| {
        b.iter(|| checked::<()>(context, &test, &add_three))
    });
}

criterion_group!(benches, call_drop_output, call);
criterion_main!(benches);
