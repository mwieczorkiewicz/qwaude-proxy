//! Criterion benchmark: the production zero-copy-targeted transform vs. a
//! deliberately naive full-deserialize/full-owned-rebuild/full-reserialize
//! baseline, at a few message-array sizes.
//!
//! `harness = false` (set in `Cargo.toml`) means this file owns its own
//! `main()`: it prints a one-shot allocation-count comparison (via a
//! counting `#[global_allocator]` — no extra dependency needed for that),
//! then drives the criterion latency benchmarks by hand
//! (`Criterion::default().configure_from_args()`), which is the documented
//! pattern for a `harness = false` bench that needs custom setup alongside
//! criterion's own timing loop.
//!
//! Per the team's affirmed Testing Posture, this benchmark is
//! tracked/reported, not a hard merge gate — bench numbers are inherently
//! noisy across machines.

// The crate's [lints.clippy] deny table is package-wide, covering this bench
// target too. `expect()` here is always on benchmark-fixture data this file
// constructs itself (never real request input), so it is allowed here the
// same way it's allowed in #[cfg(test)] modules elsewhere in the crate.
#![allow(clippy::expect_used)]

use criterion::{BenchmarkId, Criterion};
use qwaude_proxy::transform;
use serde::{Deserialize, Serialize};
use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::sync::atomic::{AtomicUsize, Ordering};

const NOTICE_PREFIX: &str = "[System Notification] ";

// --- Allocation counting (std-only; no dhat/stats_alloc dependency) -------

struct CountingAllocator;

static ALLOC_COUNT: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

fn count_allocations<F: FnOnce()>(f: F) -> usize {
    let before = ALLOC_COUNT.load(Ordering::Relaxed);
    f();
    ALLOC_COUNT.load(Ordering::Relaxed) - before
}

fn print_allocation_comparison() {
    let body = build_fixture(100);

    let zero_copy_allocs = count_allocations(|| {
        let (out, _report) = transform::coerce_system_messages(black_box(&body), NOTICE_PREFIX)
            .expect("fixture is valid json");
        black_box(out);
    });
    let targeted_serde_only_allocs = count_allocations(|| {
        let out = targeted_mutation_via_serde_json_only(black_box(&body), NOTICE_PREFIX);
        black_box(out);
    });
    let naive_value_rebuild_allocs = count_allocations(|| {
        let out = naive_full_rebuild_via_value(black_box(&body), NOTICE_PREFIX);
        black_box(out);
    });
    let naive_allocs = count_allocations(|| {
        let out = naive_coerce_system_messages(black_box(&body), NOTICE_PREFIX);
        black_box(out);
    });

    println!("allocation count (single call, 100-message fixture, ~1/5 mid-stream system):");
    println!(
        "  zero-copy targeted mutation (prod, simd-json dispatch) : {zero_copy_allocs} allocations"
    );
    println!("  targeted mutation, serde_json parse only               : {targeted_serde_only_allocs} allocations");
    println!("  naive full rebuild via Value (same representation)     : {naive_value_rebuild_allocs} allocations");
    println!(
        "  naive full rebuild via typed structs                   : {naive_allocs} allocations"
    );
    println!();
}

// --- Isolating the simd-json dispatch cost: the same targeted-mutation
// algorithm as production, but forced through serde_json's parser only (no
// simd-json, no extra owned-buffer copy) -- lets the benchmark separate
// "targeted mutation vs. full rebuild" from "simd-json dispatch overhead vs.
// plain serde_json", since transform::coerce_messages (the mutation core)
// is public and shared by both.

fn targeted_mutation_via_serde_json_only(body: &[u8], notice_prefix: &str) -> Vec<u8> {
    let mut value: serde_json::Value = serde_json::from_slice(body).expect("fixture is valid json");
    let messages = value
        .get_mut("messages")
        .and_then(serde_json::Value::as_array_mut)
        .expect("fixture has a messages array");
    let _report = transform::coerce_messages(messages, notice_prefix);
    serde_json::to_vec(&value).expect("serialize request")
}

/// The same targeted-mutation *representation* (`serde_json::Value`) as
/// production, but rebuilding -- cloning -- every message unconditionally,
/// touched or not. Isolates "targeted mutation vs. full rebuild" while
/// holding the value representation constant, separate from "Value vs.
/// typed struct" (which `naive_coerce_system_messages` below conflates it
/// with).
fn naive_full_rebuild_via_value(body: &[u8], notice_prefix: &str) -> Vec<u8> {
    let value: serde_json::Value = serde_json::from_slice(body).expect("fixture is valid json");
    let obj = value.as_object().expect("fixture is an object");

    let mut rebuilt_obj = serde_json::Map::new();
    for (key, val) in obj {
        if key != "messages" {
            rebuilt_obj.insert(key.clone(), val.clone());
        }
    }

    let messages = obj
        .get("messages")
        .and_then(serde_json::Value::as_array)
        .expect("fixture has a messages array");
    let mut rebuilt_messages = Vec::with_capacity(messages.len());
    for (idx, message) in messages.iter().enumerate() {
        let mut rebuilt_message = message.clone(); // unconditional clone, touched or not
        if idx != 0
            && rebuilt_message
                .get("role")
                .and_then(serde_json::Value::as_str)
                == Some("system")
        {
            if let Some(message_obj) = rebuilt_message.as_object_mut() {
                message_obj.insert(
                    "role".to_string(),
                    serde_json::Value::String("user".to_string()),
                );
                if let Some(serde_json::Value::String(text)) = message_obj.get_mut("content") {
                    *text = format!("{notice_prefix}{text}");
                }
            }
        }
        rebuilt_messages.push(rebuilt_message);
    }
    rebuilt_obj.insert(
        "messages".to_string(),
        serde_json::Value::Array(rebuilt_messages),
    );

    serde_json::to_vec(&serde_json::Value::Object(rebuilt_obj)).expect("serialize request")
}

// --- Naive baseline: full owned-struct deserialize, unconditional rebuild
// of every message, full reserialize. This is deliberately what a
// non-zero-copy implementation would typically look like -- every message is
// cloned into a new struct even when nothing about it changes, in contrast
// to transform::coerce_messages, which only touches messages that actually
// need coercion.

#[derive(Debug, Clone, Deserialize, Serialize)]
struct NaiveMessage {
    role: String,
    content: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct NaiveChatRequest {
    model: String,
    stream: bool,
    messages: Vec<NaiveMessage>,
}

fn naive_coerce_system_messages(body: &[u8], notice_prefix: &str) -> Vec<u8> {
    let request: NaiveChatRequest = serde_json::from_slice(body).expect("fixture is valid json");

    let mut rebuilt_messages = Vec::with_capacity(request.messages.len());
    for (idx, message) in request.messages.into_iter().enumerate() {
        // Rebuilt unconditionally, whether or not this message needs any
        // change at all -- the naive opposite of targeted mutation.
        let mut rebuilt = NaiveMessage {
            role: message.role.clone(),
            content: message.content.clone(),
        };
        if idx != 0 && rebuilt.role == "system" {
            rebuilt.role = "user".to_string();
            rebuilt.content = rebuilt.content.map(|text| format!("{notice_prefix}{text}"));
        }
        rebuilt_messages.push(rebuilt);
    }

    let rebuilt_request = NaiveChatRequest {
        model: request.model,
        stream: request.stream,
        messages: rebuilt_messages,
    };
    serde_json::to_vec(&rebuilt_request).expect("serialize naive request")
}

// --- Fixture -----------------------------------------------------------

/// A chat-completion request body with `message_count` messages: a leading
/// user message, then a mix of user/assistant messages with a system
/// message (needing coercion) every 5th slot.
fn build_fixture(message_count: usize) -> Vec<u8> {
    let mut messages = vec![serde_json::json!({
        "role": "user",
        "content": "start of a longer conversation that will go on for a while.",
    })];
    for i in 0..message_count {
        let message = if i % 5 == 0 {
            serde_json::json!({
                "role": "system",
                "content": format!(
                    "reminder number {i}: please remember the house rules and follow \
                     them carefully throughout the rest of this conversation."
                ),
            })
        } else {
            serde_json::json!({
                "role": if i % 2 == 0 { "user" } else { "assistant" },
                "content": format!(
                    "message body number {i}, with a bit of realistic length to it, \
                     just for good measure and to approximate a real payload."
                ),
            })
        };
        messages.push(message);
    }
    let request = serde_json::json!({
        "model": "qwen3.6",
        "stream": false,
        "messages": messages,
    });
    serde_json::to_vec(&request).expect("serialize fixture")
}

// --- Latency benchmark ---------------------------------------------------

fn bench_transform(c: &mut Criterion) {
    let mut group = c.benchmark_group("coerce_system_messages");
    for size in [10_usize, 100, 1_000] {
        let body = build_fixture(size);

        group.bench_with_input(
            BenchmarkId::new("zero_copy_targeted", size),
            &body,
            |b, body| {
                b.iter(|| {
                    let (out, _report) =
                        transform::coerce_system_messages(black_box(body), NOTICE_PREFIX)
                            .expect("fixture is valid json");
                    black_box(out);
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("targeted_mutation_serde_json_only", size),
            &body,
            |b, body| {
                b.iter(|| {
                    let out = targeted_mutation_via_serde_json_only(black_box(body), NOTICE_PREFIX);
                    black_box(out);
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("naive_full_rebuild_via_value", size),
            &body,
            |b, body| {
                b.iter(|| {
                    let out = naive_full_rebuild_via_value(black_box(body), NOTICE_PREFIX);
                    black_box(out);
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("naive_full_rebuild", size),
            &body,
            |b, body| {
                b.iter(|| {
                    let out = naive_coerce_system_messages(black_box(body), NOTICE_PREFIX);
                    black_box(out);
                });
            },
        );
    }
    group.finish();
}

fn main() {
    print_allocation_comparison();

    let mut criterion = Criterion::default().configure_from_args();
    bench_transform(&mut criterion);
    criterion.final_summary();
}
