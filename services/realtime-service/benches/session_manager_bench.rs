use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use realtime_service::services::session_manager::{SessionManager, Connection};
use tokio::sync::mpsc;
use uuid::Uuid;
use std::sync::Arc;

fn create_connection() -> Connection {
    let (tx, _rx) = mpsc::channel(16);
    Connection { sender: tx }
}

fn bench_insert_single(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    c.bench_function("insert_single_connection", |b| {
        b.iter(|| {
            let manager = SessionManager::new();
            let session_id = Uuid::new_v4();
            let conn_id = 1;
            
            manager.insert(
                black_box(session_id),
                black_box(conn_id),
                create_connection()
            );
        });
    });
}

fn bench_insert_multiple(c: &mut Criterion) {
    let mut group = c.benchmark_group("insert_multiple");
    
    for size in [10, 50, 100, 500, 1000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter(|| {
                let manager = SessionManager::new();
                let session_id = Uuid::new_v4();
                
                for i in 0..size {
                    manager.insert(
                        session_id,
                        i,
                        create_connection()
                    );
                }
            });
        });
    }
    
    group.finish();
}

fn bench_broadcast(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("broadcast");
    
    for conn_count in [10, 50, 100, 500].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(conn_count),
            conn_count,
            |b, &conn_count| {
                let manager = SessionManager::new();
                let session_id = Uuid::new_v4();
                
                // Setup connections
                for i in 0..conn_count {
                    manager.insert(session_id, i, create_connection());
                }
                
                b.iter(|| {
                    manager.broadcast_message(
                        black_box(session_id),
                        black_box("test message"),
                        None
                    );
                });
            }
        );
    }
    
    group.finish();
}

fn bench_concurrent_inserts(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    c.bench_function("concurrent_inserts_100", |b| {
        b.iter(|| {
            rt.block_on(async {
                let manager = Arc::new(SessionManager::new());
                let session_id = Uuid::new_v4();
                
                let mut handles = vec![];
                
                for i in 0..100 {
                    let manager_clone = Arc::clone(&manager);
                    let handle = tokio::spawn(async move {
                        manager_clone.insert(
                            session_id,
                            i,
                            create_connection()
                        );
                    });
                    handles.push(handle);
                }
                
                for handle in handles {
                    handle.await.ok();
                }
            });
        });
    });
}

fn bench_remove(c: &mut Criterion) {
    let mut group = c.benchmark_group("remove");
    
    for size in [10, 50, 100].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter_batched(
                || {
                    let manager = SessionManager::new();
                    let session_id = Uuid::new_v4();
                    
                    for i in 0..size {
                        manager.insert(session_id, i, create_connection());
                    }
                    
                    (manager, session_id)
                },
                |(manager, session_id)| {
                    for i in 0..size {
                        manager.remove(session_id, i);
                    }
                },
                criterion::BatchSize::SmallInput
            );
        });
    }
    
    group.finish();
}

fn bench_mixed_operations(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    c.bench_function("mixed_operations", |b| {
        b.iter(|| {
            let manager = SessionManager::new();
            let session_id = Uuid::new_v4();
            
            // Insert 50 connections
            for i in 0..50 {
                manager.insert(session_id, i, create_connection());
            }
            
            // Broadcast 10 messages
            for _ in 0..10 {
                manager.broadcast_message(session_id, "test", None);
            }
            
            // Remove 25 connections
            for i in 0..25 {
                manager.remove(session_id, i);
            }
            
            // Broadcast again
            for _ in 0..10 {
                manager.broadcast_message(session_id, "test", None);
            }
        });
    });
}

fn bench_session_churn(c: &mut Criterion) {
    c.bench_function("session_churn_1000", |b| {
        b.iter(|| {
            let manager = SessionManager::new();
            
            // Create and destroy 1000 sessions
            for _ in 0..1000 {
                let session_id = Uuid::new_v4();
                manager.insert(session_id, 1, create_connection());
                manager.remove(session_id, 1);
            }
        });
    });
}

fn bench_lock_contention(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    c.bench_function("lock_contention", |b| {
        b.iter(|| {
            rt.block_on(async {
                let manager = Arc::new(SessionManager::new());
                let session_id = Uuid::new_v4();
                
                let mut handles = vec![];
                
                // Spawn tasks that all try to access the same session
                for i in 0..50 {
                    let manager_clone = Arc::clone(&manager);
                    let handle = tokio::spawn(async move {
                        manager_clone.insert(session_id, i, create_connection());
                        manager_clone.broadcast_message(session_id, "test", None);
                        manager_clone.remove(session_id, i);
                    });
                    handles.push(handle);
                }
                
                for handle in handles {
                    handle.await.ok();
                }
            });
        });
    });
}

criterion_group!(
    benches,
    bench_insert_single,
    bench_insert_multiple,
    bench_broadcast,
    bench_concurrent_inserts,
    bench_remove,
    bench_mixed_operations,
    bench_session_churn,
    bench_lock_contention
);

criterion_main!(benches);