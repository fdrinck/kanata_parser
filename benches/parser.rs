use criterion::{Criterion, criterion_group, criterion_main};
use kanata::Parser;
use std::fs;
use std::hint::black_box;

fn parse_streaming_benchmark_small(c: &mut Criterion) {
    let input = fs::read("testinput/kanata-sample-1.log").unwrap();

    c.bench_function("parse/small", |b| {
        b.iter(|| {
            let mut parser = Parser::new(black_box(&input));

            for cmd in &mut parser {
                black_box(cmd).1.unwrap();
            }
        })
    });
}

fn parse_streaming_benchmark_big(c: &mut Criterion) {
    let input = fs::read("testinput/kanata-sample-2.log").unwrap();

    c.bench_function("parse/big", |b| {
        b.iter(|| {
            let mut parser = Parser::new(black_box(&input));

            for cmd in &mut parser {
                black_box(cmd).1.unwrap();
            }
        })
    });
}

criterion_group!(
    benches,
    parse_streaming_benchmark_small,
    parse_streaming_benchmark_big
);
criterion_main!(benches);
