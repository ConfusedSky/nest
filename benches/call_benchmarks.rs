use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::any::TypeId;
use wren::{context::Native, test::create_test_vm, CallHandle, GetValue, Handle, WrenType};
use wren_sys as ffi;

fn raw_ffi<'wren, G: GetValue<'wren, Native> + 'static>(
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

        if TypeId::of::<String>() == TypeId::of::<G>() {
            let mut len = std::mem::MaybeUninit::uninit();
            let res = ffi::wrenGetSlotBytes(context, 0, len.as_mut_ptr()).cast();
            let len = len.assume_init().try_into().unwrap();
            let slice = std::slice::from_raw_parts(res, len);
            let _str = String::from_utf8_lossy(slice).to_string();
        }
        if TypeId::of::<f64>() == TypeId::of::<G>() {
            let _res = ffi::wrenGetSlotDouble(context, 0);
        }
    }
}

fn raw<'wren, G: GetValue<'wren, Native> + 'static>(
    context: &mut wren::test::Context<'wren, Native>,
    test: &Handle<'wren>,
    add_three: &CallHandle<'wren>,
    slot_type: WrenType,
) {
    unsafe {
        context
            .call_raw::<G, _>(
                test,
                add_three,
                &(
                    &black_box(1.0_f64),
                    &black_box(2.0_f64),
                    &black_box(3.0_f64),
                ),
                slot_type,
            )
            .unwrap();
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
            static add_three_string(a, b, c) { (a+b+c).toString }
            static add_three(a, b, c) { a+b+c }
        }",
        |_| {},
    );
    let context = vm.get_context();
    let add_three = context.make_call_handle(wren::cstr!("add_three_string(_,_,_)"));

    let context_ptr = context.as_ptr();
    let test_ptr = test.as_ptr();
    let add_three_ptr = add_three.as_ptr();

    let mut group = c.benchmark_group("Call");
    group.bench_function("Raw FFI String", |b| {
        b.iter(|| raw_ffi::<String>(context_ptr, test_ptr, add_three_ptr));
    });

    group.bench_function("Raw FFI Number", |b| {
        b.iter(|| raw_ffi::<f64>(context_ptr, test_ptr, add_three_ptr));
    });

    group.bench_function("Unchecked Raw String", |b| {
        b.iter(|| raw::<String>(context, &test, &add_three, WrenType::String))
    });

    group.bench_function("Unchecked Raw Number", |b| {
        b.iter(|| raw::<f64>(context, &test, &add_three, WrenType::Num))
    });

    group.bench_function("Unchecked String", |b| {
        b.iter(|| unchecked::<String>(context, &test, &add_three))
    });

    group.bench_function("Unchecked Number", |b| {
        b.iter(|| unchecked::<f64>(context, &test, &add_three))
    });

    group.bench_function("Checked String", |b| {
        b.iter(|| checked::<String>(context, &test, &add_three))
    });

    group.bench_function("Checked Number", |b| {
        b.iter(|| checked::<String>(context, &test, &add_three))
    });

    group.bench_function("No VM String", |b| {
        b.iter(|| (black_box(1) + black_box(2) + black_box(3)).to_string())
    });

    group.bench_function("No VM Number", |b| {
        b.iter(|| (black_box(1) + black_box(2) + black_box(3)).to_string())
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

    group.bench_function("Unchecked Raw", |b| {
        b.iter(|| raw::<()>(context, &test, &add_three, WrenType::Unknown))
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
