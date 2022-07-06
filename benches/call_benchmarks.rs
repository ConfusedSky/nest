use criterion::{black_box, criterion_group, criterion_main, Criterion};
use wren::test::create_test_vm;
use wren_sys as ffi;

pub fn criterion_benchmark(c: &mut Criterion) {
    let (mut vm, test) = create_test_vm(
        "class Test {
            static add_three(a, b, c) { (a+b+c).toString }
        }",
        |_| {},
    );
    let context = vm.get_context();
    let add_three = context.make_call_handle(wren::cstr!("add_three(_,_,_)"));

    c.bench_function("Test call Raw FFI", |b| {
        b.iter(|| unsafe {
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
        });
    });

    c.bench_function("Test Call Unchecked", |b| {
        b.iter(|| unsafe {
            let res = context
                .call_unchecked::<String, _>(
                    &test,
                    &add_three,
                    &(
                        &black_box(1.0_f64),
                        &black_box(2.0_f64),
                        &black_box(3.0_f64),
                    ),
                )
                .unwrap();
            assert!(res == "6");
        })
    });
    c.bench_function("Test Call Checked", |b| {
        b.iter(|| {
            let res = context
                .call::<String, _>(
                    &test,
                    &add_three,
                    &(
                        &black_box(1.0_f64),
                        &black_box(2.0_f64),
                        &black_box(3.0_f64),
                    ),
                )
                .unwrap();
            assert!(res == "6");
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
