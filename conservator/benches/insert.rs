use conservator::Creatable;
use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use std::sync::atomic::{AtomicI32, Ordering};

mod common;
use common::{CreateUser, User, create_test_pool};

fn bench_insert_returning_pk(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let pool = rt.block_on(create_test_pool());

    let counter = AtomicI32::new(0);
    c.bench_function("insert_returning_pk", |b| {
        b.to_async(&rt).iter(|| async {
            let count = counter.fetch_add(1, Ordering::SeqCst);
            let user = CreateUser::sample(count);
            let pk = user.insert::<User>().returning_pk(&pool).await.unwrap();
            black_box(pk);
        })
    });
}

fn bench_insert_returning_entity(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let pool = rt.block_on(create_test_pool());

    let counter = AtomicI32::new(0);
    c.bench_function("insert_returning_entity", |b| {
        b.to_async(&rt).iter(|| async {
            let count = counter.fetch_add(1, Ordering::SeqCst);
            let user = CreateUser::sample(count);
            let entity = user.insert::<User>().returning_entity(&pool).await.unwrap();
            black_box(entity);
        })
    });
}

fn bench_batch_insert(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    let mut group = c.benchmark_group("batch_insert");

    for batch_size in [10, 50, 100].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(batch_size),
            batch_size,
            |b, &size| {
                let pool = rt.block_on(create_test_pool());
                let counter = AtomicI32::new(0);

                b.to_async(&rt).iter(|| async {
                    for _ in 0..size {
                        let count = counter.fetch_add(1, Ordering::SeqCst);
                        let user = CreateUser::sample(count);
                        user.insert::<User>().returning_pk(&pool).await.unwrap();
                    }
                })
            },
        );
    }

    group.finish();
}

fn bench_insert_with_transaction(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let pool = rt.block_on(create_test_pool());

    let counter = AtomicI32::new(0);
    c.bench_function("insert_with_transaction", |b| {
        b.to_async(&rt).iter(|| async {
            let mut conn = pool.get().await.unwrap();
            let tx = conn.begin().await.unwrap();

            for _ in 0..10 {
                let count = counter.fetch_add(1, Ordering::SeqCst);
                let user = CreateUser::sample(count);
                user.insert::<User>().returning_pk(&tx).await.unwrap();
            }

            tx.commit().await.unwrap();
        })
    });
}

criterion_group!(
    benches,
    bench_insert_returning_pk,
    bench_insert_returning_entity,
    bench_batch_insert,
    bench_insert_with_transaction
);
criterion_main!(benches);
