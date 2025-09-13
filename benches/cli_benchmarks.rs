use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use std::time::Duration;
use tempfile::TempDir;
use tokio::runtime::Runtime;
use zephyrfs_cli::{Config, ZephyrClient};

fn config_benchmarks(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    c.bench_function("config_default_creation", |b| {
        b.iter(|| Config::default())
    });

    let temp_dir = tempfile::tempdir().unwrap();
    let config = Config::default();
    let config_path = temp_dir.path().join("bench_config.yaml");
    
    // Benchmark config save
    c.bench_function("config_save_yaml", |b| {
        b.iter(|| {
            config.save(Some(black_box(config_path.to_str().unwrap()))).unwrap();
        })
    });

    // Ensure config file exists for load benchmark
    config.save(Some(config_path.to_str().unwrap())).unwrap();

    // Benchmark config load
    c.bench_function("config_load_yaml", |b| {
        b.iter(|| {
            Config::load(Some(black_box(config_path.to_str().unwrap()))).unwrap();
        })
    });
}

fn file_operation_benchmarks(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let temp_dir = tempfile::tempdir().unwrap();
    
    // Test different file sizes
    let sizes = vec![1024, 10_240, 102_400, 1_048_576]; // 1KB, 10KB, 100KB, 1MB
    
    for size in sizes {
        let data = vec![0u8; size];
        let file_path = temp_dir.path().join(format!("test_{}.bin", size));
        
        c.bench_with_input(
            BenchmarkId::new("file_write", size),
            &(file_path.clone(), data.clone()),
            |b, (path, data)| {
                b.to_async(&rt).iter(|| async {
                    tokio::fs::write(black_box(path), black_box(data)).await.unwrap();
                });
            }
        );
        
        // Create file for read benchmark
        rt.block_on(tokio::fs::write(&file_path, &data)).unwrap();
        
        c.bench_with_input(
            BenchmarkId::new("file_read", size),
            &file_path,
            |b, path| {
                b.to_async(&rt).iter(|| async {
                    tokio::fs::read(black_box(path)).await.unwrap();
                });
            }
        );
    }
}

fn throughput_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("throughput");
    
    // Config serialization throughput
    let config = Config::default();
    
    group.bench_function("yaml_serialization", |b| {
        b.iter(|| {
            serde_yaml::to_string(black_box(&config)).unwrap();
        })
    });
    
    group.bench_function("json_serialization", |b| {
        b.iter(|| {
            serde_json::to_string(black_box(&config)).unwrap();
        })
    });
    
    group.bench_function("toml_serialization", |b| {
        b.iter(|| {
            toml::to_string(black_box(&config)).unwrap();
        })
    });
    
    // Deserialization benchmarks
    let yaml_data = serde_yaml::to_string(&config).unwrap();
    let json_data = serde_json::to_string(&config).unwrap();
    let toml_data = toml::to_string(&config).unwrap();
    
    group.bench_function("yaml_deserialization", |b| {
        b.iter(|| {
            serde_yaml::from_str::<Config>(black_box(&yaml_data)).unwrap();
        })
    });
    
    group.bench_function("json_deserialization", |b| {
        b.iter(|| {
            serde_json::from_str::<Config>(black_box(&json_data)).unwrap();
        })
    });
    
    group.bench_function("toml_deserialization", |b| {
        b.iter(|| {
            toml::from_str::<Config>(black_box(&toml_data)).unwrap();
        })
    });
    
    group.finish();
}

fn client_benchmarks(c: &mut Criterion) {
    let config = Config::default();
    
    c.bench_function("client_creation", |b| {
        b.iter(|| {
            ZephyrClient::new(black_box(&config));
        })
    });
}

fn memory_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory");
    group.measurement_time(Duration::from_secs(10));
    
    // Benchmark memory usage for different operations
    group.bench_function("config_clones", |b| {
        let config = Config::default();
        b.iter(|| {
            let _clones: Vec<Config> = (0..1000).map(|_| config.clone()).collect();
        })
    });
    
    group.bench_function("large_file_simulation", |b| {
        b.iter(|| {
            // Simulate processing a large file in chunks
            let chunk_size = 1024 * 1024; // 1MB chunks
            let num_chunks = 100;
            
            for _ in 0..num_chunks {
                let _chunk = vec![0u8; chunk_size];
                black_box(_chunk);
            }
        })
    });
    
    group.finish();
}

fn hash_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("hashing");
    
    let data_sizes = vec![1024, 10_240, 102_400, 1_048_576]; // 1KB to 1MB
    
    for size in data_sizes {
        let data = vec![0u8; size];
        
        group.throughput(Throughput::Bytes(size as u64));
        
        group.bench_with_input(
            BenchmarkId::new("sha256", size),
            &data,
            |b, data| {
                use sha2::{Sha256, Digest};
                b.iter(|| {
                    let mut hasher = Sha256::new();
                    hasher.update(black_box(data));
                    hasher.finalize();
                })
            }
        );
    }
    
    group.finish();
}

criterion_group!(
    benches,
    config_benchmarks,
    file_operation_benchmarks,
    throughput_benchmarks,
    client_benchmarks,
    memory_benchmarks,
    hash_benchmarks
);
criterion_main!(benches);