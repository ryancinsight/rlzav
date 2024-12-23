use criterion::{black_box, criterion_group, criterion_main, Criterion};
use lzav::{lzav_compress, lzav_compress_bound, lzav_decompress};
use rand::Rng;

fn generate_test_data(size: usize) -> Vec<u8> {
    let mut rng = rand::thread_rng();
    let mut data = vec![0u8; size];
    rng.fill(&mut data[..]);
    data
}

fn compression_benchmark(c: &mut Criterion) {
    let data = generate_test_data(1024 * 1024); // 1MB of random data
    let bound = lzav_compress_bound(data.len() as i32) as usize;
    let mut compressed = vec![0u8; bound];
    
    c.bench_function("compress 1MB", |b| {
        b.iter(|| {
            lzav_compress(
                black_box(&data),
                black_box(&mut compressed),
                None,
            )
        })
    });
}

fn decompression_benchmark(c: &mut Criterion) {
    let data = generate_test_data(1024 * 1024);
    let bound = lzav_compress_bound(data.len() as i32) as usize;
    let mut compressed = vec![0u8; bound];
    let compressed_size = lzav_compress(&data, &mut compressed, None).unwrap();
    compressed.truncate(compressed_size);
    
    let mut decompressed = vec![0u8; data.len()];
    
    c.bench_function("decompress 1MB", |b| {
        b.iter(|| {
            lzav_decompress(
                black_box(&compressed),
                black_box(&mut decompressed),
                black_box(data.len()),
            )
        })
    });
}

criterion_group!(benches, compression_benchmark, decompression_benchmark);
criterion_main!(benches); 