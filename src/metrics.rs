use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metrics {
    pub commands: CommandMetrics,
    pub network: NetworkMetrics,
    pub storage: StorageMetrics,
    pub performance: PerformanceMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandMetrics {
    pub total_commands: u64,
    pub commands_by_type: HashMap<String, u64>,
    pub command_durations: HashMap<String, Vec<f64>>, // in milliseconds
    pub failed_commands: u64,
    pub last_command_time: Option<u64>, // unix timestamp
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkMetrics {
    pub total_requests: u64,
    pub failed_requests: u64,
    pub average_response_time: f64, // milliseconds
    pub timeouts: u64,
    pub bytes_uploaded: u64,
    pub bytes_downloaded: u64,
    pub peer_connections: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageMetrics {
    pub files_uploaded: u64,
    pub files_downloaded: u64,
    pub total_bytes_processed: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub disk_operations: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub memory_usage_mb: f64,
    pub cpu_usage_percent: f64,
    pub startup_time_ms: f64,
    pub gc_collections: u32,
    pub peak_memory_mb: f64,
}

#[derive(Debug)]
pub struct MetricsCollector {
    metrics: Arc<RwLock<Metrics>>,
    start_time: Instant,
}

impl Default for Metrics {
    fn default() -> Self {
        Self {
            commands: CommandMetrics {
                total_commands: 0,
                commands_by_type: HashMap::new(),
                command_durations: HashMap::new(),
                failed_commands: 0,
                last_command_time: None,
            },
            network: NetworkMetrics {
                total_requests: 0,
                failed_requests: 0,
                average_response_time: 0.0,
                timeouts: 0,
                bytes_uploaded: 0,
                bytes_downloaded: 0,
                peer_connections: 0,
            },
            storage: StorageMetrics {
                files_uploaded: 0,
                files_downloaded: 0,
                total_bytes_processed: 0,
                cache_hits: 0,
                cache_misses: 0,
                disk_operations: 0,
            },
            performance: PerformanceMetrics {
                memory_usage_mb: 0.0,
                cpu_usage_percent: 0.0,
                startup_time_ms: 0.0,
                gc_collections: 0,
                peak_memory_mb: 0.0,
            },
        }
    }
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(RwLock::new(Metrics::default())),
            start_time: Instant::now(),
        }
    }

    pub async fn record_command_start(&self, command: &str) -> CommandTimer {
        let mut metrics = self.metrics.write().await;
        metrics.commands.total_commands += 1;
        *metrics.commands.commands_by_type.entry(command.to_string()).or_insert(0) += 1;
        metrics.commands.last_command_time = Some(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        );

        CommandTimer {
            command: command.to_string(),
            start_time: Instant::now(),
            collector: self.metrics.clone(),
        }
    }

    pub async fn record_network_request(&self, duration: Duration, success: bool, bytes_transferred: u64, upload: bool) {
        let mut metrics = self.metrics.write().await;
        metrics.network.total_requests += 1;
        
        if !success {
            metrics.network.failed_requests += 1;
        }

        // Update average response time (simple moving average)
        let duration_ms = duration.as_secs_f64() * 1000.0;
        let total_requests = metrics.network.total_requests as f64;
        metrics.network.average_response_time = 
            (metrics.network.average_response_time * (total_requests - 1.0) + duration_ms) / total_requests;

        if upload {
            metrics.network.bytes_uploaded += bytes_transferred;
        } else {
            metrics.network.bytes_downloaded += bytes_transferred;
        }
    }

    pub async fn record_timeout(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.network.timeouts += 1;
        metrics.network.failed_requests += 1;
    }

    pub async fn record_file_upload(&self, file_size: u64) {
        let mut metrics = self.metrics.write().await;
        metrics.storage.files_uploaded += 1;
        metrics.storage.total_bytes_processed += file_size;
        metrics.storage.disk_operations += 1;
    }

    pub async fn record_file_download(&self, file_size: u64) {
        let mut metrics = self.metrics.write().await;
        metrics.storage.files_downloaded += 1;
        metrics.storage.total_bytes_processed += file_size;
        metrics.storage.disk_operations += 1;
    }

    pub async fn record_cache_hit(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.storage.cache_hits += 1;
    }

    pub async fn record_cache_miss(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.storage.cache_misses += 1;
    }

    pub async fn update_peer_count(&self, peer_count: u32) {
        let mut metrics = self.metrics.write().await;
        metrics.network.peer_connections = peer_count;
    }

    pub async fn record_memory_usage(&self, memory_mb: f64) {
        let mut metrics = self.metrics.write().await;
        metrics.performance.memory_usage_mb = memory_mb;
        if memory_mb > metrics.performance.peak_memory_mb {
            metrics.performance.peak_memory_mb = memory_mb;
        }
    }

    pub async fn record_cpu_usage(&self, cpu_percent: f64) {
        let mut metrics = self.metrics.write().await;
        metrics.performance.cpu_usage_percent = cpu_percent;
    }

    pub async fn get_metrics(&self) -> Metrics {
        let metrics = self.metrics.read().await;
        let mut result = metrics.clone();
        result.performance.startup_time_ms = self.start_time.elapsed().as_secs_f64() * 1000.0;
        result
    }

    pub async fn get_summary(&self) -> MetricsSummary {
        let metrics = self.get_metrics().await;
        
        MetricsSummary {
            uptime_seconds: self.start_time.elapsed().as_secs(),
            total_commands: metrics.commands.total_commands,
            success_rate: if metrics.commands.total_commands > 0 {
                ((metrics.commands.total_commands - metrics.commands.failed_commands) as f64 
                 / metrics.commands.total_commands as f64) * 100.0
            } else { 0.0 },
            network_success_rate: if metrics.network.total_requests > 0 {
                ((metrics.network.total_requests - metrics.network.failed_requests) as f64
                 / metrics.network.total_requests as f64) * 100.0
            } else { 0.0 },
            average_response_time_ms: metrics.network.average_response_time,
            total_bytes_transferred: metrics.network.bytes_uploaded + metrics.network.bytes_downloaded,
            files_processed: metrics.storage.files_uploaded + metrics.storage.files_downloaded,
            cache_hit_rate: if metrics.storage.cache_hits + metrics.storage.cache_misses > 0 {
                (metrics.storage.cache_hits as f64 / 
                 (metrics.storage.cache_hits + metrics.storage.cache_misses) as f64) * 100.0
            } else { 0.0 },
            memory_usage_mb: metrics.performance.memory_usage_mb,
            peak_memory_mb: metrics.performance.peak_memory_mb,
        }
    }

    pub async fn export_metrics(&self) -> Result<String, serde_json::Error> {
        let metrics = self.get_metrics().await;
        serde_json::to_string_pretty(&metrics)
    }

    pub async fn reset_metrics(&self) {
        let mut metrics = self.metrics.write().await;
        *metrics = Metrics::default();
    }
}

#[derive(Debug)]
pub struct CommandTimer {
    command: String,
    start_time: Instant,
    collector: Arc<RwLock<Metrics>>,
}

impl Drop for CommandTimer {
    fn drop(&mut self) {
        let duration = self.start_time.elapsed();
        let duration_ms = duration.as_secs_f64() * 1000.0;
        
        // Use a blocking task to update metrics since we're in Drop
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let mut metrics = self.collector.write().await;
                metrics.commands.command_durations
                    .entry(self.command.clone())
                    .or_insert_with(Vec::new)
                    .push(duration_ms);
            });
        });
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MetricsSummary {
    pub uptime_seconds: u64,
    pub total_commands: u64,
    pub success_rate: f64,
    pub network_success_rate: f64,
    pub average_response_time_ms: f64,
    pub total_bytes_transferred: u64,
    pub files_processed: u64,
    pub cache_hit_rate: f64,
    pub memory_usage_mb: f64,
    pub peak_memory_mb: f64,
}

// Global metrics collector instance
lazy_static::lazy_static! {
    pub static ref METRICS: MetricsCollector = MetricsCollector::new();
}

// Convenience macros for metrics collection
#[macro_export]
macro_rules! time_command {
    ($command:expr) => {
        crate::metrics::METRICS.record_command_start($command).await
    };
}

#[macro_export]
macro_rules! record_network_success {
    ($duration:expr, $bytes:expr, $upload:expr) => {
        crate::metrics::METRICS.record_network_request($duration, true, $bytes, $upload).await
    };
}

#[macro_export]
macro_rules! record_network_failure {
    ($duration:expr, $bytes:expr, $upload:expr) => {
        crate::metrics::METRICS.record_network_request($duration, false, $bytes, $upload).await
    };
}