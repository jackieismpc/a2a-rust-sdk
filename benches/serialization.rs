use a2a_rust_sdk::models::{AgentMessage, MessagePart, MessageRole};
use criterion::{criterion_group, criterion_main, Criterion};

fn bench_agent_message(c: &mut Criterion) {
    let message = AgentMessage {
        message_id: "msg-1".to_string(),
        context_id: Some("ctx-1".to_string()),
        task_id: Some("task-1".to_string()),
        role: MessageRole::User,
        parts: vec![MessagePart::Text { text: "hello".to_string() }],
    };

    c.bench_function("agent_message_serialize", |b| {
        b.iter(|| serde_json::to_string(&message).expect("serialize"))
    });

    let serialized = serde_json::to_string(&message).expect("serialize");
    c.bench_function("agent_message_deserialize", |b| {
        b.iter(|| serde_json::from_str::<AgentMessage>(&serialized).expect("deserialize"))
    });
}

criterion_group!(benches, bench_agent_message);
criterion_main!(benches);
