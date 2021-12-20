use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use route::Graph;

pub fn route_benchmark(c: &mut Criterion) {
    let graph = Graph::new_from_binfile("../graph.bin");
    let nodes = graph.generate_random_water_nodes(10);

    let mut group = c.benchmark_group("route");
    group.sample_size(10);
    for (i, (start_node, end_node)) in nodes.iter().enumerate() {
        group.bench_with_input(
            BenchmarkId::from_parameter(i),
            &(start_node, end_node),
            |b, &(start_node, end_node)| {
                b.iter(|| graph.dijkstra(*start_node, *end_node));
            },
        );
    }
    group.finish();
}

criterion_group!(benches, route_benchmark);
criterion_main!(benches);
