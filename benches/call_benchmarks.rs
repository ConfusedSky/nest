use criterion::{black_box, criterion_group, criterion_main, Criterion};
use wren::{context::Native, test::create_test_vm, CallHandle, Handle};
use wren_sys as ffi;

fn raw_ffi(context: &mut wren::test::Context<Native>, test: &Handle, add_three: &CallHandle) {
    unsafe {
        let test_ptr = test.as_ptr();
        let context_ptr = context.as_ptr();
        let add_three_ptr = add_three.as_ptr();
        ffi::wrenEnsureSlots(context_ptr, 4);

        ffi::wrenSetSlotHandle(context_ptr, 0, test_ptr);
        ffi::wrenSetSlotDouble(context_ptr, 1, black_box(1.0_f64));
        ffi::wrenSetSlotDouble(context_ptr, 2, black_box(2.0_f64));
        ffi::wrenSetSlotDouble(context_ptr, 3, black_box(3.0_f64));

        let result = ffi::wrenCall(context_ptr, add_three_ptr);

        match result {
            ffi::WrenInterpretResult_WREN_RESULT_SUCCESS => (),
            _ => panic!("Interpret failed"),
        }

        let mut len = std::mem::MaybeUninit::uninit();
        let res = ffi::wrenGetSlotBytes(context_ptr, 0, len.as_mut_ptr()).cast();
        let len = len.assume_init().try_into().unwrap();
        let slice = std::slice::from_raw_parts(res, len);
        let str = String::from_utf8_lossy(slice).to_string();
        assert!(str == "6")
    }
}

fn unchecked<'wren>(
    context: &mut wren::test::Context<'wren, Native>,
    test: &Handle<'wren>,
    add_three: &CallHandle<'wren>,
) {
    unsafe {
        let res = context
            .call_unchecked::<String, _>(
                test,
                add_three,
                &(
                    &black_box(1.0_f64),
                    &black_box(2.0_f64),
                    &black_box(3.0_f64),
                ),
            )
            .unwrap();
        assert!(res == "6");
    }
}

fn checked<'wren>(
    context: &mut wren::test::Context<'wren, Native>,
    test: &Handle<'wren>,
    add_three: &CallHandle<'wren>,
) {
    let res = context
        .call::<String, _>(
            test,
            add_three,
            &(
                &black_box(1.0_f64),
                &black_box(2.0_f64),
                &black_box(3.0_f64),
            ),
        )
        .unwrap();
    assert!(res == "6");
}

pub fn criterion_benchmark(c: &mut Criterion) {
    let (mut vm, test) = create_test_vm(
        "class Test {
            static add_three(a, b, c) { (a+b+c).toString }
        }",
        |_| {},
    );
    let context = vm.get_context();
    let add_three = context.make_call_handle(wren::cstr!("add_three(_,_,_)"));

    let mut group = c.benchmark_group("Call");

    group.bench_function("Raw FFI", |b| {
        b.iter(|| raw_ffi(context, &test, &add_three));
    });

    group.bench_function("Unchecked", |b| {
        b.iter(|| unchecked(context, &test, &add_three))
    });

    group.bench_function("Checked", |b| {
        b.iter(|| checked(context, &test, &add_three))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
