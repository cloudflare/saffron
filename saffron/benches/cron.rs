use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};

fn cron_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("Cron.from_str");
    let inputs = ["* * * * *", "1 12 3 6 *", "12-35 1-23 2-5 1-11 *"];
    for input in inputs.iter() {
        group.bench_with_input(BenchmarkId::from_parameter(input), input, |b, input| {
            b.iter(|| input.parse::<saffron::Cron>().unwrap())
        });
    }
    group.finish()
}

criterion_group!(benches, cron_benchmark);
criterion_main!(benches);
