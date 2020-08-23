use bounded_collections::{
    bounded_quadmap::quad,
    BoundedBitmap,
    BoundedQuadmap,
};
use criterion::{
    criterion_group,
    criterion_main,
    Criterion,
};

criterion_group!(bench_solve, bench_bounded_bitwise_map_set, bench_bounded_bitwise_map_get);
criterion_main!(bench_solve);

fn bench_bounded_bitwise_map_get(c: &mut Criterion) {
    let mut g = c.benchmark_group("bounded_bitwise_maps::get");
    let len = 100_000;
    let quad_map = <BoundedQuadmap<usize, quad>>::with_len(len);
    let bit_map = <BoundedBitmap<usize, bool>>::with_len(len);
    let vec_bool = vec![false; len];
    g.bench_function("BoundedQuadmap", |bencher| {
        bencher.iter(|| {
            for i in 0..len {
                assert_eq!(quad_map.get(i), Ok(quad::B00));
            }
        })
    });
    g.bench_function("BoundedBitmap", |bencher| {
        bencher.iter(|| {
            for i in 0..len {
                assert_eq!(bit_map.get(i), Ok(false));
            }
        })
    });
    g.bench_function("Vec<u8> (reference)", |bencher| {
        bencher.iter(|| {
            for i in 0..len {
                assert!(!vec_bool[i]);
            }
        })
    });
}

fn bench_bounded_bitwise_map_set(c: &mut Criterion) {
    let mut g = c.benchmark_group("bounded_bitwise_maps::set");
    let len = 100_000;
    let mut quad_map = <BoundedQuadmap<usize, quad>>::with_len(len);
    let mut bit_map = <BoundedBitmap<usize, bool>>::with_len(len);
    let mut vec_bool = vec![false; len];
    g.bench_function("BoundedQuadmap", |bencher| {
        bencher.iter(|| {
            for i in 0..len {
                quad_map.set(i, quad::B11).unwrap();
            }
        })
    });
    g.bench_function("BoundedBitmap", |bencher| {
        bencher.iter(|| {
            for i in 0..len {
                bit_map.set(i, true).unwrap();
            }
        })
    });
    g.bench_function("Vec<u8> (reference)", |bencher| {
        bencher.iter(|| {
            for i in 0..len {
                vec_bool[i] = true;
            }
        })
    });
}
