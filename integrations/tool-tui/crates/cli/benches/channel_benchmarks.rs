//! Performance benchmarks for messaging channels

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use dx_cli::channels::{MessageQueue, QueuedMessage};

fn bench_message_enqueue(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("message_enqueue", |b| {
        b.to_async(&rt).iter(|| async {
            let queue = MessageQueue::new();
            let msg = QueuedMessage::new(
                black_box("telegram".to_string()),
                black_box("user123".to_string()),
                black_box("Hello".to_string()),
            );
            queue.enqueue(msg).await;
        });
    });
}

fn bench_message_dequeue(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("message_dequeue", |b| {
        b.to_async(&rt).iter(|| async {
            let queue = MessageQueue::new();
            let msg = QueuedMessage::new(
                "telegram".to_string(),
                "user123".to_string(),
                "Hello".to_string(),
            );
            queue.enqueue(msg).await;
            black_box(queue.dequeue().await);
        });
    });
}

criterion_group!(benches, bench_message_enqueue, bench_message_dequeue);
criterion_main!(benches);
