use criterion::{black_box, criterion_group, criterion_main, Criterion};
use wren::test::create_test_vm;

pub fn criterion_benchmark(c: &mut Criterion) {
    let (mut vm, test) = create_test_vm(
        "class Test {
            static add_three(a, b, c) { (a+b+c).toString }
        }",
        |_| {},
    );
    let context = vm.get_context();
    let add_three = context.make_call_handle(wren::cstr!("add_three(_,_,_)"));

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
