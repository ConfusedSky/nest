use criterion::{black_box, criterion_group, criterion_main, Criterion};

#[inline]
fn fibonacci(n: u64) -> u64 {
    // match n {
    // 0 => 1,
    // 1 => 1,
    // n => fibonacci(n - 1) + fibonacci(n - 2),
    // }
    // let mut n1 = 0;
    // let mut n2 = 1;
    // for _i in 1..n {
    // let sum = n1 + n2;
    // n1 = n2;
    // n2 = sum;
    // }

    // n2
    fn help(a: u64, b: u64, n: u64) -> u64 {
        if n == 0 {
            a
        } else {
            help(b, a + b, n - 1)
        }
    }
    help(0, 1, n)
}

pub fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("fib 20", |b| b.iter(|| fibonacci(black_box(20))));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
