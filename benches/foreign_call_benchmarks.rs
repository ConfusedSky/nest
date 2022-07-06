use criterion::{black_box, criterion_group, criterion_main, Criterion};
use wren::{
    context::{Foreign, Native},
    test::create_test_vm,
    CallHandle, GetValue, Handle, WrenType,
};
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

        // This is a hack to make sure that () doesn't get handle the return path
        if std::mem::size_of::<G>() != 0 {
            let mut len = std::mem::MaybeUninit::uninit();
            let res = ffi::wrenGetSlotBytes(context, 0, len.as_mut_ptr()).cast();
            let len = len.assume_init().try_into().unwrap();
            let slice = std::slice::from_raw_parts(res, len);
            let _str = String::from_utf8_lossy(slice).to_string();
        }
    }
}

unsafe fn unchecked(mut context: wren::test::Context<Foreign>) {
    let (_, a, b, c) = context.get_stack_unchecked::<((), f64, f64, f64)>();

    context.set_return_value(&(a + b + c));
}

fn checked(mut context: wren::test::Context<Foreign>) {
    let (_, a, b, c) = context.get_stack::<((), f64, f64, f64)>();
    let (a, b, c) = (a.unwrap(), b.unwrap(), c.unwrap());

    context.set_return_value(&(a + b + c));
}

fn callback<'wren>(
    context: &mut wren::test::Context<'wren, Native>,
    test: &Handle<'wren>,
    add_three: &CallHandle<'wren>,
) {
    let result = context.call::<f64, _>(test, add_three, &()).unwrap();
    assert_eq!(result, 6.0_f64);
}

pub fn foreign_call(c: &mut Criterion) {
    // let (mut vm, test) = create_test_vm(
    // "class Test {
    // static add_three() { (a+b+c).toString }
    // foreign static add_three_(a, b, c)
    // }",
    // |_| {},
    // );
    // let context = vm.get_context();
    // let add_three = context.make_call_handle(wren::cstr!("add_three()"));

    // let context_ptr = context.as_ptr();
    // let test_ptr = test.as_ptr();
    // let add_three_ptr = add_three.as_ptr();

    let mut group = c.benchmark_group("Foreign Call");
    // group.bench_function("Raw FFI", |b| {
    // b.iter(|| raw_ffi::<String>(context_ptr, test_ptr, add_three_ptr));
    // });

    // group.bench_function("Unchecked Raw", |b| {
    // b.iter(|| raw::<String>(context, &test, &add_three, WrenType::String))
    // });

    let (mut vm, test) = create_test_vm(
        "class Test {
            static add_three() { add_three_(1, 2, 3) }
            foreign static add_three_(a, b, c)
        }",
        |vm| vm.set_static_foreign_method("add_three_(_,_,_)", unchecked),
    );
    let context = vm.get_context();
    let add_three = context.make_call_handle(wren::cstr!("add_three()"));

    group.bench_function("Unchecked", |b| {
        b.iter(|| callback(context, &test, &add_three))
    });

    let (mut vm, test) = create_test_vm(
        "class Test {
            static add_three() { add_three_(1, 2, 3) }
            foreign static add_three_(a, b, c)
        }",
        |vm| vm.set_static_foreign_method("add_three_(_,_,_)", checked),
    );
    let context = vm.get_context();
    let add_three = context.make_call_handle(wren::cstr!("add_three()"));

    group.bench_function("Checked", |b| {
        b.iter(|| callback(context, &test, &add_three))
    });
}

criterion_group!(benches, foreign_call);
criterion_main!(benches);
