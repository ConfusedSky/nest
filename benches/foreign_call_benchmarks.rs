use criterion::{
    criterion_group, criterion_main, measurement::WallTime, BenchmarkGroup, Criterion, Throughput,
};
use wren::{
    context::{Foreign, Native},
    test::{create_test_vm, UserData},
    CallHandle, ForeignMethod, GetValue, Handle,
};
use wren_sys as ffi;

unsafe fn raw_ffi(context: wren::test::Context<Foreign>) {
    let context = context.as_ptr();
    let a = ffi::wrenGetSlotDouble(context, 1);
    let b = ffi::wrenGetSlotDouble(context, 2);
    let c = ffi::wrenGetSlotDouble(context, 3);
    // ffi::wrenSetSlotDouble(context, 0, a + b + c);
    let str = (a + b + c).to_string();
    ffi::wrenSetSlotBytes(
        context,
        0,
        str.as_ptr().cast(),
        str.len().try_into().unwrap(),
    );
}

unsafe fn unchecked_raw(mut context: wren::test::Context<Foreign>) {
    let a = f64::get_slot_raw(&mut context, 1, wren::WrenType::Num);
    let b = f64::get_slot_raw(&mut context, 2, wren::WrenType::Num);
    let c = f64::get_slot_raw(&mut context, 3, wren::WrenType::Num);

    context.set_return_value(&(a + b + c).to_string());
}

unsafe fn unchecked(mut context: wren::test::Context<Foreign>) {
    let (_, a, b, c) = context.get_stack_unchecked::<((), f64, f64, f64)>();

    context.set_return_value(&(a + b + c).to_string());
}

fn checked(mut context: wren::test::Context<Foreign>) {
    let (_, a, b, c) = context.get_stack::<((), f64, f64, f64)>();
    let (a, b, c) = (a.unwrap(), b.unwrap(), c.unwrap());

    context.set_return_value(&(a + b + c).to_string());
}

fn callback<'wren>(
    context: &mut wren::test::Context<'wren, Native>,
    test: &Handle<'wren>,
    add_three: &CallHandle<'wren>,
) {
    let result = context.call::<bool, _>(test, add_three, &()).unwrap();
    assert!(result);
}

fn setup<'wren>(
    group: &mut BenchmarkGroup<WallTime>,
    name: &str,
    foreign: ForeignMethod<'wren, UserData<'wren>>,
    is_multi: bool,
) {
    let (mut vm, test) = create_test_vm(
        "class Test {
            static add_three() { add_three_(1, 2, 3) == \"6\" }
            static add_three_wrapped() { add_three_wrapper(1, 2, 3) == \"6\" }
            static add_three_multi() {
                for (i in 0..100) {
                    add_three_(i+1, i+2, i+3)
                }
                return true
            }
            static add_three_multi_wrapped() {
                for (i in 0..1000) {
                    add_three_wrapper(i+1, i+2, i+3)
                }
                return true
            }
            static add_three_wrapper(a, b, c) {
                if ( !(a is Num) || !(b is Num) || !(c is Num)) {
                    Fiber.abort(\"All arguments must be numbers\")
                }
                return add_three_(a, b, c)
            }
            foreign static add_three_(a, b, c)
        }",
        |vm| vm.set_static_foreign_method("add_three_(_,_,_)", foreign),
    );
    let context = vm.get_context();
    let add_three = context.make_call_handle(wren::cstr!("add_three()"));
    let add_three_wrapped = context.make_call_handle(wren::cstr!("add_three_wrapped()"));
    let _add_three_multi_wrapped =
        context.make_call_handle(wren::cstr!("add_three_multi_wrapped()"));
    let add_three_multi = context.make_call_handle(wren::cstr!("add_three_multi()"));

    if is_multi {
        group.bench_function(&(name.to_string() + " Multi"), |b| {
            b.iter(|| callback(context, &test, &add_three_multi))
        });
        // group.bench_function(&(name.to_string() + " Multi Wrapped"), |b| {
        // b.iter(|| callback(context, &test, &add_three_multi_wrapped))
        // });
    } else {
        group.bench_function(name, |b| b.iter(|| callback(context, &test, &add_three)));
        group.bench_function(&(name.to_string() + " Wrapped"), |b| {
            b.iter(|| callback(context, &test, &add_three_wrapped))
        });
    }
}

pub fn foreign_call(c: &mut Criterion) {
    let mut group = c.benchmark_group("Foreign Call");
    setup(&mut group, "Raw FFI", raw_ffi, false);
    setup(&mut group, "Unchecked Raw", unchecked_raw, false);
    setup(&mut group, "Unchecked", unchecked, false);
    setup(&mut group, "Checked", checked, false);
}

pub fn foreign_call_multi(c: &mut Criterion) {
    let mut group = c.benchmark_group("Foreign Call Multi");
    setup(&mut group, "Raw FFI", raw_ffi, true);
    setup(&mut group, "Unchecked Raw", unchecked_raw, true);
    setup(&mut group, "Unchecked", unchecked, true);
    setup(&mut group, "Checked", checked, true);
}

criterion_group!(benches, foreign_call, foreign_call_multi);
criterion_main!(benches);
