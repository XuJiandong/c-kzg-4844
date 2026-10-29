use blob_tree::{verify, Tree};
use criterion::{
    black_box, criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion, SamplingMode,
    Throughput,
};
use std::time::Duration;

fn input(len: usize) -> Vec<Vec<u8>> {
    (0..len as u64).map(|i| i.to_be_bytes().to_vec()).collect()
}

fn commitment(c: &mut Criterion) {
    // Initialize the shared trusted setup before measuring tree construction.
    c_kzg::ethereum_kzg_settings(0);
    let mut group = c.benchmark_group("commitment");
    group.sampling_mode(SamplingMode::Flat);
    for len in [10_000, 100_000, 1_000_000] {
        let values = input(len);
        group.throughput(Throughput::Elements(len as u64));
        group.bench_with_input(BenchmarkId::from_parameter(len), &values, |b, values| {
            // Drop each generated tree outside the timer without retaining a batch.
            b.iter_batched(
                || (),
                |()| Tree::new(black_box(values)).unwrap(),
                BatchSize::PerIteration,
            );
        });
    }
    group.finish();
}

fn proof(c: &mut Criterion) {
    const TREE_LEN: usize = 100_000;
    let tree = Tree::new(&input(TREE_LEN)).unwrap();
    let mut group = c.benchmark_group("proof/100000");
    group.sampling_mode(SamplingMode::Flat);
    for count in [10, 100, 1_000] {
        // Spread queries across the tree, matching the large CKB fixture at 1,000 leaves.
        let indices = (0..count).map(|i| i * TREE_LEN / count).collect::<Vec<_>>();
        let (values, proof) = tree.prove(&indices).unwrap();
        assert!(verify(tree.root(), &values, &proof).unwrap());
        group.throughput(Throughput::Elements(count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(count),
            &indices,
            |b, indices| {
                b.iter_batched(
                    || (),
                    |()| tree.prove(black_box(indices)).unwrap(),
                    BatchSize::PerIteration,
                );
            },
        );
    }
    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .sample_size(10)
        .warm_up_time(Duration::from_secs(1))
        .measurement_time(Duration::from_secs(5));
    targets = commitment, proof
}
criterion_main!(benches);
