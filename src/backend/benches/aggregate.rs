use backend::{
    deserialize::{
        iterators::{ContainsIterator, NaiveIterator},
        reader::Reader,
    },
    population::PopulationDataset,
    Dataset,
};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use geo::{Coordinate, Rect}; // Replace with your actual crate name

fn generate_progressive_queries() -> Vec<Rect<f32>> {
    let mut queries = Vec::new();

    // Start with a small rectangle and progressively grow it
    let steps = 20; // Number of progressive steps

    for i in 1..=steps {
        let progress = i as f32 / steps as f32;

        // Progressive bounds: start small and grow to world bounds
        let min_x = -180.0 * progress;
        let max_x = 180.0 * progress;
        let min_y = -90.0 * progress;
        let max_y = 90.0 * progress;

        let rect = Rect::new(
            Coordinate { x: min_x, y: min_y },
            Coordinate { x: max_x, y: max_y },
        );

        queries.push(rect);
    }

    queries
}

fn generate_sample_data() -> memmap2::Mmap {
    // Replace this with your actual data generation logic
    // This is a placeholder that should create your serialized spatial data
    let file = std::fs::File::open("population.db").unwrap();
    unsafe { memmap2::Mmap::map(&file).unwrap() }
}

fn benchmark_contains_iterator(c: &mut Criterion) {
    let data = generate_sample_data();
    let queries = generate_progressive_queries();

    let mut group = c.benchmark_group("contains_iterator");

    for (i, query) in queries.iter().enumerate() {
        let query_size = format!(
            "query_{:02}_bounds_{:.1}_{:.1}_{:.1}_{:.1}",
            i,
            query.min().x,
            query.min().y,
            query.max().x,
            query.max().y
        );

        group.bench_with_input(
            BenchmarkId::new("ContainsIterator", &query_size),
            query,
            |b, query| {
                b.iter(|| {
                    let reader = Reader::new(&data);
                    let mut iterator = ContainsIterator::<
                        <PopulationDataset as Dataset>::Type,
                        <PopulationDataset as Dataset>::AggregateType,
                        _,
                    >::new(reader, *query);
                    let mut count = 0;
                    while let Some(tile) = iterator.next() {
                        black_box(tile);
                        count += 1;
                    }
                    (count, iterator.nodes_visited)
                })
            },
        );
    }

    group.finish();
}

fn benchmark_naive_iterator(c: &mut Criterion) {
    let data = generate_sample_data();
    let queries = generate_progressive_queries();

    let mut group = c.benchmark_group("naive_iterator");

    for (i, query) in queries.iter().enumerate() {
        let query_size = format!(
            "query_{:02}_bounds_{:.1}_{:.1}_{:.1}_{:.1}",
            i,
            query.min().x,
            query.min().y,
            query.max().x,
            query.max().y
        );

        group.bench_with_input(
            BenchmarkId::new("NaiveIterator", &query_size),
            query,
            |b, query| {
                b.iter(|| {
                    let reader = Reader::new(&data);
                    let mut iterator = NaiveIterator::<
                        <PopulationDataset as Dataset>::Type,
                        <PopulationDataset as Dataset>::AggregateType,
                        _,
                    >::new(reader, *query);
                    let mut count = 0;
                    while let Some(tile) = iterator.next() {
                        black_box(tile);
                        count += 1;
                    }
                    (count, iterator.nodes_visited)
                })
            },
        );
    }

    group.finish();
}

fn benchmark_comparison(c: &mut Criterion) {
    let data = generate_sample_data();
    let queries = generate_progressive_queries();

    let mut group = c.benchmark_group("iterator_comparison");

    // Sample a few key query sizes for direct comparison
    let sample_indices = [5, 10, 15, 19]; // Small, medium, large, world-size

    for &idx in &sample_indices {
        let query = queries[idx];
        let query_name = format!("size_{}", idx);

        let mut iters = 0;
        group.bench_with_input(BenchmarkId::new("contains", &idx), &query, |b, query| {
            b.iter(|| {
                iters += 1;
                let reader = Reader::new(&data);
                let mut iterator = ContainsIterator::<
                    <PopulationDataset as Dataset>::Type,
                    <PopulationDataset as Dataset>::AggregateType,
                    _,
                >::new(reader, *query);
                // let mut count = 0;
                while let Some(tile) = iterator.next() {
                    black_box(tile);
                    // count += 1;
                }
                if iters == 1 {
                    println!("\ncontains: nodes visited: {}", iterator.nodes_visited);
                }
                // count
            })
        });

        let mut iters = 0;
        group.bench_with_input(BenchmarkId::new("naive", &idx), &query, |b, query| {
            b.iter(|| {
                iters += 1;
                let reader = Reader::new(&data);
                let mut iterator = NaiveIterator::<
                    <PopulationDataset as Dataset>::Type,
                    <PopulationDataset as Dataset>::AggregateType,
                    _,
                >::new(reader, *query);
                let mut count = 0;
                while let Some(tile) = iterator.next() {
                    black_box(tile);
                    // count += 1;
                }

                if iters == 1 {
                    println!("\nnaive: nodes visited: {}", iterator.nodes_visited);
                }
                // count
            })
        });
    }

    group.finish();
}

// Optional: Benchmark that measures nodes visited efficiency
fn benchmark_efficiency(c: &mut Criterion) {
    let data = generate_sample_data();
    let queries = generate_progressive_queries();

    let mut group = c.benchmark_group("iterator_efficiency");

    for (i, query) in queries.iter().enumerate() {
        if i % 4 != 0 {
            continue;
        } // Sample every 4th query to reduce benchmark time

        let query_name = format!("query_{}", i);

        // Measure ContainsIterator efficiency
        group.bench_with_input(
            BenchmarkId::new("contains_efficiency", &query_name),
            query,
            |b, query| {
                b.iter_custom(|iters| {
                    let start = std::time::Instant::now();
                    let mut total_nodes = 0;
                    let mut total_results = 0;

                    for _ in 0..iters {
                        let reader = Reader::new(&data);
                        let mut iterator = ContainsIterator::<
                            <PopulationDataset as Dataset>::Type,
                            <PopulationDataset as Dataset>::AggregateType,
                            _,
                        >::new(reader, *query);
                        let mut count = 0;
                        while let Some(_) = iterator.next() {
                            count += 1;
                        }
                        total_nodes += iterator.nodes_visited;
                        total_results += count;
                    }

                    // Print efficiency metrics occasionally
                    if iters == 1 {
                        println!(
                            "ContainsIterator query_{}: {} nodes visited, {} results",
                            i, total_nodes, total_results
                        );
                    }

                    start.elapsed()
                })
            },
        );

        // Measure NaiveIterator efficiency
        group.bench_with_input(
            BenchmarkId::new("naive_efficiency", &query_name),
            query,
            |b, query| {
                b.iter_custom(|iters| {
                    let start = std::time::Instant::now();
                    let mut total_nodes = 0;
                    let mut total_results = 0;

                    for _ in 0..iters {
                        let reader = Reader::new(&data);
                        let mut iterator = NaiveIterator::<
                            <PopulationDataset as Dataset>::Type,
                            <PopulationDataset as Dataset>::AggregateType,
                            _,
                        >::new(reader, *query);
                        let mut count = 0;
                        while let Some(_) = iterator.next() {
                            count += 1;
                        }
                        total_nodes += iterator.nodes_visited;
                        total_results += count;
                    }

                    // Print efficiency metrics occasionally
                    if iters == 1 {
                        println!(
                            "NaiveIterator query_{}: {} nodes visited, {} results",
                            i, total_nodes, total_results
                        );
                    }

                    start.elapsed()
                })
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    // benchmark_contains_iterator,
    // benchmark_naive_iterator,
    benchmark_comparison,
    // benchmark_efficiency
);
criterion_main!(benches);
