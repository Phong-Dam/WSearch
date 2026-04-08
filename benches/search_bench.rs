//! Search benchmark using Criterion
//!
//! Run with: cargo bench --bench search_bench 2>&1 | tee bench.log

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::sync::Arc;

/// Dummy search function for benchmarking
fn search_prefix(files: &[String], query: &str) -> Vec<usize> {
    let q = query.to_lowercase();
    files.iter()
        .enumerate()
        .filter(|(_, name)| name.to_lowercase().starts_with(&q))
        .map(|(i, _)| i)
        .collect()
}

/// Create test file names
fn create_test_files(count: usize) -> Vec<String> {
    (0..count)
        .map(|i| format!("file_{}.txt", i))
        .collect()
}

fn benchmark_search(c: &mut Criterion) {
    let files = create_test_files(100_000);
    let files = Arc::new(files);

    c.bench_function("search_prefix_100k", |b| {
        b.iter(|| {
            let query = black_box("file_");
            let _results = search_prefix(&files, query);
        })
    });
}

fn benchmark_search_longer(c: &mut Criterion) {
    let files = create_test_files(500_000);
    let files = Arc::new(files);

    c.bench_function("search_prefix_500k", |b| {
        b.iter(|| {
            let query = black_box("document_");
            let _results = search_prefix(&files, query);
        })
    });
}

fn benchmark_binary_search(c: &mut Criterion) {
    let mut files: Vec<String> = (0..100_000).map(|i| format!("{:08}", i)).collect();
    files.sort();

    c.bench_function("binary_search_100k", |b| {
        b.iter(|| {
            let query = black_box("00500");
            let q = query.to_lowercase();
            let idx = files.binary_search_by(|f| f.to_lowercase().cmp(&q));
            let _ = idx;
        })
    });
}

fn benchmark_fuzzy_match(c: &mut Criterion) {
    let files: Vec<String> = (0..10_000)
        .map(|i| format!("document_{}_important.txt", i))
        .collect();

    c.bench_function("fuzzy_match_10k", |b| {
        b.iter(|| {
            let pattern = black_box("doc_imp");
            for file in &files {
                let _ = fuzzy_match_simple(&file.to_lowercase(), pattern);
            }
        })
    });
}

/// Simple fuzzy matching for benchmarking
fn fuzzy_match_simple(text: &str, pattern: &str) -> bool {
    let text_chars: Vec<char> = text.chars().collect();
    let pattern_chars: Vec<char> = pattern.chars().collect();

    if pattern_chars.is_empty() {
        return true;
    }

    let mut pattern_idx = 0;
    for ch in text_chars.iter() {
        if pattern_idx < pattern_chars.len() && *ch == pattern_chars[pattern_idx] {
            pattern_idx += 1;
            if pattern_idx == pattern_chars.len() {
                return true;
            }
        }
    }
    false
}

fn benchmark_sort(c: &mut Criterion) {
    let files: Vec<(String, u32)> = (0..50_000)
        .map(|i| (format!("file_{}.txt", i), i % 100))
        .collect();

    c.bench_function("sort_50k_by_count", |b| {
        b.iter(|| {
            let mut sorted = files.clone();
            sorted.sort_by(|a, b| b.1.cmp(&a.1));
        })
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(10);
    targets = benchmark_search, benchmark_search_longer, benchmark_binary_search, benchmark_fuzzy_match, benchmark_sort
}
criterion_main!(benches);