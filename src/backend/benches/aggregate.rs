use backend::{
    deserialize::{
        iterators::{ContainsIterator, NaiveIterator},
        reader::Reader,
    },
    population::PopulationDataset,
    Dataset,
};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use geo::{Area, Coord, Rect};

fn generate_queries() -> Vec<Rect<f32>> {
    let steps = 10;

    fn halve(rect: &Rect<f32>) -> Rect<f32> {
        let scale_factor = (0.5_f32).sqrt(); // ≈ 0.7071

        let center_x = (rect.min().x + rect.max().x) / 2.0;
        let center_y = (rect.min().y + rect.max().y) / 2.0;

        let half_width = (rect.max().x - rect.min().x) / 2.0 * scale_factor;
        let half_height = (rect.max().y - rect.min().y) / 2.0 * scale_factor;

        Rect::new(
            Coord {
                x: center_x - half_width,
                y: center_y - half_height,
            },
            Coord {
                x: center_x + half_width,
                y: center_y + half_height,
            },
        )
    }

    let mut out = vec![Rect::new(
        Coord { x: -180., y: -90. },
        Coord { x: 180., y: 90. },
    )];

    for i in 0..steps {
        let previous = &out[i];
        let next = halve(previous);
        out.push(next);
    }

    out
}

fn open_database() -> memmap2::Mmap {
    let file = std::fs::File::open("population.db").unwrap();
    unsafe { memmap2::Mmap::map(&file).unwrap() }
}

fn benchmark_iterator_performance(c: &mut Criterion) {
    let data = open_database();
    let queries = generate_queries();

    let mut group = c.benchmark_group("Queries");

    for &query in &queries {
        let area = query.unsigned_area();

        group.bench_with_input(BenchmarkId::new("Quadtree", area), &query, |b, query| {
            b.iter(|| {
                let reader = Reader::new(&data);
                let mut iterator = ContainsIterator::<
                    <PopulationDataset as Dataset>::Type,
                    <PopulationDataset as Dataset>::AggregateType,
                    _,
                >::new(reader, *query);

                while let Some(tile) = iterator.next() {
                    black_box(tile);
                }
            })
        });

        group.bench_with_input(BenchmarkId::new("Naive", area), &query, |b, query| {
            b.iter(|| {
                let reader = Reader::new(&data);
                let mut iterator = NaiveIterator::<
                    <PopulationDataset as Dataset>::Type,
                    <PopulationDataset as Dataset>::AggregateType,
                    _,
                >::new(reader, *query);

                while let Some(tile) = iterator.next() {
                    black_box(tile);
                }
            })
        });
    }

    group.finish();
}

criterion_group!(benches, benchmark_iterator_performance);
criterion_main!(benches);
