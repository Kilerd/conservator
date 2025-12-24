use conservator::Domain;
use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};

mod common;
use common::{User, create_test_pool, populate_sample_data};

fn bench_select_by_pk(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let pool = rt.block_on(async {
        let pool = create_test_pool().await;
        populate_sample_data(&pool, 100).await;
        pool
    });

    c.bench_function("select_by_pk", |b| {
        b.to_async(&rt).iter(|| async {
            let user = User::fetch_one_by_pk(&black_box(42), &pool).await.unwrap();
            black_box(user);
        })
    });
}

fn bench_select_all(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    let mut group = c.benchmark_group("select_all");

    for count in [10, 50, 100].iter() {
        let pool = rt.block_on(async {
            let pool = create_test_pool().await;
            populate_sample_data(&pool, *count).await;
            pool
        });

        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, _| {
            b.to_async(&rt).iter(|| async {
                let users = User::select().all(&pool).await.unwrap();
                black_box(users);
            })
        });
    }

    group.finish();
}

fn bench_select_with_filter(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let pool = rt.block_on(async {
        let pool = create_test_pool().await;
        populate_sample_data(&pool, 100).await;
        pool
    });

    c.bench_function("select_with_filter", |b| {
        b.to_async(&rt).iter(|| async {
            let users = User::select()
                .filter(User::COLUMNS.age.gt(30))
                .all(&pool)
                .await
                .unwrap();
            black_box(users);
        })
    });
}

fn bench_select_with_complex_filter(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let pool = rt.block_on(async {
        let pool = create_test_pool().await;
        populate_sample_data(&pool, 100).await;
        pool
    });

    c.bench_function("select_with_complex_filter", |b| {
        b.to_async(&rt).iter(|| async {
            let users = User::select()
                .filter(User::COLUMNS.age.gt(25).and(User::COLUMNS.age.lt(50)))
                .all(&pool)
                .await
                .unwrap();
            black_box(users);
        })
    });
}

fn bench_select_optional(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let pool = rt.block_on(async {
        let pool = create_test_pool().await;
        populate_sample_data(&pool, 100).await;
        pool
    });

    c.bench_function("select_optional_found", |b| {
        b.to_async(&rt).iter(|| async {
            let user = User::select()
                .filter(User::COLUMNS.id.eq(42))
                .optional(&pool)
                .await
                .unwrap();
            black_box(user);
        })
    });

    c.bench_function("select_optional_not_found", |b| {
        b.to_async(&rt).iter(|| async {
            let user = User::select()
                .filter(User::COLUMNS.id.eq(99999))
                .optional(&pool)
                .await
                .unwrap();
            black_box(user);
        })
    });
}

criterion_group!(
    benches,
    bench_select_by_pk,
    bench_select_all,
    bench_select_with_filter,
    bench_select_with_complex_filter,
    bench_select_optional
);
criterion_main!(benches);
