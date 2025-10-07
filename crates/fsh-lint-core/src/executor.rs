//! Parallel execution engine for FSH linting operations
//!
//! This module provides the core execution engine that coordinates parallel
//! processing of FSH files through the linting pipeline.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{
    Arc, Mutex, Once,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use std::time::{Duration, Instant};

use rayon::prelude::*;
use tracing::{Level, debug, error, info, span, warn};

/// Initialize the global Rayon thread pool once
static THREAD_POOL_INIT: Once = Once::new();

fn init_global_thread_pool(threads: usize) {
    THREAD_POOL_INIT.call_once(|| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .thread_name(|index| format!("fsh-lint-worker-{index}"))
            .build_global()
            .unwrap_or_else(|e| {
                warn!(
                    "Could not configure global thread pool (may already be initialized): {}",
                    e
                );
            });
        info!(
            "Configured global rayon thread pool with {} threads",
            threads
        );
    });
}

use crate::{
    CompiledRule, Diagnostic, FshLintConfiguration, FshLintError, Result, RuleEngine,
    SemanticAnalyzer,
};

/// Progress reporting callback type
pub type ProgressCallback = Arc<dyn Fn(ProgressInfo) + Send + Sync>;

/// Information about execution progress
#[derive(Debug, Clone)]
pub struct ProgressInfo {
    /// Total number of files to process
    pub total_files: usize,
    /// Number of files completed
    pub completed_files: usize,
    /// Current file being processed (if available)
    pub current_file: Option<PathBuf>,
    /// Elapsed time since start
    pub elapsed: Duration,
    /// Estimated time remaining (if available)
    pub estimated_remaining: Option<Duration>,
}

impl ProgressInfo {
    /// Calculate completion percentage (0.0 to 1.0)
    pub fn completion_percentage(&self) -> f64 {
        if self.total_files == 0 {
            1.0
        } else {
            self.completed_files as f64 / self.total_files as f64
        }
    }
}

/// Resource usage statistics
#[derive(Debug, Clone, Default)]
pub struct ResourceStats {
    /// Peak memory usage in bytes
    pub peak_memory_bytes: u64,
    /// Current memory usage in bytes
    pub current_memory_bytes: u64,
    /// Number of active threads
    pub active_threads: usize,
    /// Total CPU time used
    pub cpu_time: Duration,
    /// Number of files processed
    pub files_processed: usize,
    /// Average processing time per file
    pub avg_processing_time: Duration,
}

/// Resource monitor for tracking system resource usage
#[derive(Debug)]
pub struct ResourceMonitor {
    /// Memory usage samples
    memory_samples: Arc<Mutex<Vec<u64>>>,
    /// Monitoring interval
    interval: Duration,
    /// Shutdown flag for stopping the background thread
    shutdown: Arc<AtomicBool>,
}

impl ResourceMonitor {
    /// Create a new resource monitor
    pub fn new(interval: Duration) -> Self {
        Self {
            memory_samples: Arc::new(Mutex::new(Vec::new())),
            interval,
            shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Signal the monitor to shut down
    pub fn shutdown(&self) {
        self.shutdown.store(true, Ordering::Relaxed);
    }

    /// Start monitoring resources in the background
    pub fn start_monitoring(&self) -> Arc<Mutex<ResourceStats>> {
        let stats = Arc::new(Mutex::new(ResourceStats::default()));
        let stats_clone = Arc::clone(&stats);
        let samples_clone = Arc::clone(&self.memory_samples);
        let interval = self.interval;
        let shutdown = Arc::clone(&self.shutdown);

        std::thread::spawn(move || {
            while !shutdown.load(Ordering::Relaxed) {
                let current_memory = get_current_memory_usage();

                // Update samples
                {
                    let mut samples = samples_clone.lock().unwrap();
                    samples.push(current_memory);
                    // Keep only last 100 samples to prevent unbounded growth
                    if samples.len() > 100 {
                        samples.remove(0);
                    }
                }

                // Update stats
                {
                    let mut stats = stats_clone.lock().unwrap();
                    stats.current_memory_bytes = current_memory;
                    if current_memory > stats.peak_memory_bytes {
                        stats.peak_memory_bytes = current_memory;
                    }
                    stats.active_threads = rayon::current_num_threads();
                }

                std::thread::sleep(interval);
            }
        });

        stats
    }

    /// Get memory usage trend (increasing, decreasing, stable)
    pub fn get_memory_trend(&self) -> MemoryTrend {
        let samples = self.memory_samples.lock().unwrap();
        if samples.len() < 3 {
            return MemoryTrend::Stable;
        }

        let recent_samples = &samples[samples.len().saturating_sub(5)..];
        let first = recent_samples[0];
        let last = recent_samples[recent_samples.len() - 1];

        let change_percent = if first > 0 {
            ((last as f64 - first as f64) / first as f64) * 100.0
        } else {
            0.0
        };

        if change_percent > 10.0 {
            MemoryTrend::Increasing
        } else if change_percent < -10.0 {
            MemoryTrend::Decreasing
        } else {
            MemoryTrend::Stable
        }
    }
}

/// Memory usage trend
#[derive(Debug, Clone, PartialEq)]
pub enum MemoryTrend {
    Increasing,
    Decreasing,
    Stable,
}

/// Backpressure controller for managing resource usage
#[derive(Debug)]
pub struct BackpressureController {
    /// Current parallelism level
    current_parallelism: Arc<Mutex<usize>>,
    /// Maximum parallelism level
    max_parallelism: usize,
    /// Minimum parallelism level
    min_parallelism: usize,
    /// Memory threshold for applying backpressure (bytes)
    memory_threshold: Option<u64>,
}

impl BackpressureController {
    /// Create a new backpressure controller
    pub fn new(max_parallelism: usize, memory_threshold: Option<u64>) -> Self {
        Self {
            current_parallelism: Arc::new(Mutex::new(max_parallelism)),
            max_parallelism,
            min_parallelism: 1,
            memory_threshold,
        }
    }

    /// Adjust parallelism based on resource usage
    pub fn adjust_parallelism(&self, stats: &ResourceStats, trend: MemoryTrend) -> usize {
        let mut current = self.current_parallelism.lock().unwrap();

        // Check memory pressure
        if let Some(threshold) = self.memory_threshold {
            if stats.current_memory_bytes > threshold {
                // Reduce parallelism under memory pressure
                *current = (*current / 2).max(self.min_parallelism);
                warn!(
                    "Reducing parallelism to {} due to memory pressure",
                    *current
                );
            } else if stats.current_memory_bytes < threshold / 2 && trend == MemoryTrend::Decreasing
            {
                // Increase parallelism when memory usage is low and decreasing
                *current = (*current * 2).min(self.max_parallelism);
                debug!(
                    "Increasing parallelism to {} due to low memory usage",
                    *current
                );
            }
        }

        *current
    }

    /// Get current parallelism level
    pub fn get_current_parallelism(&self) -> usize {
        *self.current_parallelism.lock().unwrap()
    }
}

/// Execution context containing configuration and shared resources
pub struct ExecutionContext {
    /// Linting configuration
    pub config: FshLintConfiguration,
    /// Compiled rules for execution
    pub rules: Vec<CompiledRule>,
    /// Thread pool configuration
    pub thread_pool_size: Option<usize>,
    /// Progress reporting callback
    pub progress_callback: Option<ProgressCallback>,
    /// Memory limit in bytes (None for no limit)
    pub memory_limit: Option<u64>,
    /// Resource usage statistics
    pub resource_stats: Arc<Mutex<ResourceStats>>,
    /// Resource monitor for tracking system resources
    pub resource_monitor: Option<ResourceMonitor>,
    /// Backpressure controller for managing resource usage
    pub backpressure_controller: Option<BackpressureController>,
}

impl ExecutionContext {
    /// Create a new execution context
    pub fn new(config: FshLintConfiguration, rules: Vec<CompiledRule>) -> Self {
        Self {
            config,
            rules,
            thread_pool_size: None,
            progress_callback: None,
            memory_limit: None,
            resource_stats: Arc::new(Mutex::new(ResourceStats::default())),
            resource_monitor: None,
            backpressure_controller: None,
        }
    }

    /// Set the thread pool size
    pub fn with_thread_pool_size(mut self, size: usize) -> Self {
        self.thread_pool_size = Some(size);
        self
    }

    /// Set progress callback
    pub fn with_progress_callback(mut self, callback: ProgressCallback) -> Self {
        self.progress_callback = Some(callback);
        self
    }

    /// Set memory limit
    pub fn with_memory_limit(mut self, limit: u64) -> Self {
        self.memory_limit = Some(limit);
        self
    }

    /// Enable resource monitoring
    pub fn with_resource_monitoring(mut self, interval: Duration) -> Self {
        self.resource_monitor = Some(ResourceMonitor::new(interval));
        self
    }

    /// Enable backpressure control
    pub fn with_backpressure_control(
        mut self,
        max_parallelism: usize,
        memory_threshold: Option<u64>,
    ) -> Self {
        self.backpressure_controller = Some(BackpressureController::new(
            max_parallelism,
            memory_threshold,
        ));
        self
    }

    /// Get current resource statistics
    pub fn get_resource_stats(&self) -> ResourceStats {
        self.resource_stats.lock().unwrap().clone()
    }
}

/// Result of executing linting on a single file
#[derive(Debug)]
pub struct FileExecutionResult {
    /// Path of the processed file
    pub file_path: PathBuf,
    /// Diagnostics found in the file
    pub diagnostics: Vec<Diagnostic>,
    /// Execution time for this file
    pub execution_time: Duration,
    /// Any error that occurred during processing
    pub error: Option<FshLintError>,
}

/// Trait for executing linting operations
pub trait Executor {
    /// Execute linting on multiple files in parallel
    fn execute_parallel(&self, files: Vec<PathBuf>) -> Result<Vec<FileExecutionResult>>;

    /// Execute linting on a single file
    fn execute_single(&self, file: &Path) -> Result<FileExecutionResult>;

    /// Set the parallelism level (number of threads)
    fn set_parallelism(&mut self, threads: usize);

    /// Get current parallelism level
    fn get_parallelism(&self) -> usize;
}

/// Default implementation of the parallel executor
pub struct DefaultExecutor {
    /// Execution context
    context: ExecutionContext,
    /// Semantic analyzer instance
    semantic_analyzer: Box<dyn SemanticAnalyzer + Send + Sync>,
    /// Rule engine instance
    rule_engine: Box<dyn RuleEngine + Send + Sync>,
}

impl DefaultExecutor {
    /// Create a new default executor
    pub fn new(
        context: ExecutionContext,
        semantic_analyzer: Box<dyn SemanticAnalyzer + Send + Sync>,
        rule_engine: Box<dyn RuleEngine + Send + Sync>,
    ) -> Self {
        // Determine optimal thread count
        let thread_count = context.thread_pool_size.unwrap_or_else(|| {
            std::thread::available_parallelism()
                .map(|p| p.get())
                .unwrap_or(4)
        });

        info!("Initializing executor with {} threads", thread_count);

        // Initialize global rayon thread pool once
        init_global_thread_pool(thread_count);

        Self {
            context,
            semantic_analyzer,
            rule_engine,
        }
    }

    /// Process a single file through the complete linting pipeline
    fn process_file(&self, file_path: &Path) -> FileExecutionResult {
        let start_time = Instant::now();
        let span = span!(Level::DEBUG, "process_file", file = %file_path.display());
        let _enter = span.enter();

        debug!("Processing file: {}", file_path.display());

        // Read and parse the file
        let content = match fs::read_to_string(file_path) {
            Ok(content) => content,
            Err(io_error) => {
                let error = FshLintError::io_error(file_path, io_error);
                error!("Failed to read file {}: {}", file_path.display(), error);
                return FileExecutionResult {
                    file_path: file_path.to_path_buf(),
                    diagnostics: vec![],
                    execution_time: start_time.elapsed(),
                    error: Some(error),
                };
            }
        };
        let parse_result = match crate::FshParser::parse_content(&content) {
            Ok(result) => result,
            Err(error) => {
                error!("Failed to parse file {}: {}", file_path.display(), error);
                return FileExecutionResult {
                    file_path: file_path.to_path_buf(),
                    diagnostics: vec![],
                    execution_time: start_time.elapsed(),
                    error: Some(error),
                };
            }
        };

        let parse_result = parse_result;
        let parse_source = parse_result.source;
        let parse_errors = parse_result.errors;
        let cst = parse_result.cst;

        let mut parse_diagnostics = Vec::new();
        for parse_error in parse_errors {
            parse_diagnostics.push(Diagnostic::from_parse_error(parse_error, file_path));
        }

        // Perform semantic analysis using CST
        let mut diagnostics = parse_diagnostics;

        let semantic_result =
            self.semantic_analyzer
                .analyze(&cst, &parse_source, file_path.to_path_buf());

        match semantic_result {
            Ok(semantic_model) => {
                // Execute rules against the semantic model
                let rule_diagnostics = self.rule_engine.execute_rules(&semantic_model);
                diagnostics.extend(rule_diagnostics);
            }
            Err(e) => {
                error!(
                    "Semantic analysis failed for {}: {}",
                    file_path.display(),
                    e
                );
                // Continue with parse diagnostics only
            }
        }

        // Sort diagnostics for deterministic output
        diagnostics.sort_by(|a, b| {
            a.location
                .line
                .cmp(&b.location.line)
                .then_with(|| a.location.column.cmp(&b.location.column))
                .then_with(|| a.rule_id.cmp(&b.rule_id))
        });

        let execution_time = start_time.elapsed();
        debug!(
            "Completed processing file {} in {:?}",
            file_path.display(),
            execution_time
        );

        FileExecutionResult {
            file_path: file_path.to_path_buf(),
            diagnostics,
            execution_time,
            error: None,
        }
    }

    /// Report progress if callback is configured
    fn report_progress(&self, info: ProgressInfo) {
        if let Some(ref callback) = self.context.progress_callback {
            callback(info);
        }
    }
}

impl Executor for DefaultExecutor {
    fn execute_parallel(&self, files: Vec<PathBuf>) -> Result<Vec<FileExecutionResult>> {
        let total_files = files.len();
        let start_time = Instant::now();

        // Start resource monitoring if enabled
        let monitor_handle = self
            .context
            .resource_monitor
            .as_ref()
            .map(|monitor| monitor.start_monitoring());

        let completed_count = Arc::new(AtomicUsize::new(0));

        let results: Result<Vec<_>> = files
            .par_iter()
            .enumerate()
            .map(|(index, file_path)| {
                let result = self.process_file(file_path);

                // Update progress with atomic counter (no locks!)
                let completed = completed_count.fetch_add(1, Ordering::Relaxed) + 1;

                let elapsed = start_time.elapsed();
                let estimated_remaining = if completed > 0 {
                    let avg_time_per_file = elapsed / completed as u32;
                    let remaining_files = total_files - completed;
                    Some(avg_time_per_file * remaining_files as u32)
                } else {
                    None
                };

                // Report progress (no locks inside par_iter!)
                self.report_progress(ProgressInfo {
                    total_files,
                    completed_files: completed,
                    current_file: Some(file_path.clone()),
                    elapsed,
                    estimated_remaining,
                });

                Ok((index, result))
            })
            .collect();

        let mut indexed_results = results?;

        // Sort by original index to maintain deterministic ordering
        indexed_results.sort_by_key(|(index, _)| *index);

        let final_results: Vec<FileExecutionResult> = indexed_results
            .into_iter()
            .map(|(_, result)| result)
            .collect();

        let total_time = start_time.elapsed();
        info!(
            "Completed parallel execution of {} files in {:?}",
            total_files, total_time
        );

        // Update final statistics (outside parallel section, no contention)
        if let Ok(mut stats) = self.context.resource_stats.lock() {
            stats.files_processed = total_files;
            stats.avg_processing_time = if total_files > 0 {
                total_time / total_files as u32
            } else {
                Duration::ZERO
            };
        }

        // Final progress report
        self.report_progress(ProgressInfo {
            total_files,
            completed_files: total_files,
            current_file: None,
            elapsed: total_time,
            estimated_remaining: Some(Duration::ZERO),
        });

        // Shutdown resource monitor if it was started
        if let Some(_stats) = monitor_handle {
            if let Some(monitor) = &self.context.resource_monitor {
                monitor.shutdown();
            }
        }

        Ok(final_results)
    }

    fn execute_single(&self, file: &Path) -> Result<FileExecutionResult> {
        info!("Executing single file: {}", file.display());
        Ok(self.process_file(file))
    }

    fn set_parallelism(&mut self, threads: usize) {
        self.context.thread_pool_size = Some(threads);
    }

    fn get_parallelism(&self) -> usize {
        self.context.thread_pool_size.unwrap_or_else(|| {
            std::thread::available_parallelism()
                .map(|p| p.get())
                .unwrap_or(1)
        })
    }
}

impl DefaultExecutor {
    /// Get the execution context (for testing)
    pub fn get_context(&self) -> &ExecutionContext {
        &self.context
    }
}

/// Get current memory usage in bytes
/// This is a simplified implementation - in production you might want to use
/// a more sophisticated memory monitoring solution
fn get_current_memory_usage() -> u64 {
    // Try to get memory usage from /proc/self/status on Linux
    #[cfg(target_os = "linux")]
    {
        if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
            for line in status.lines() {
                if line.starts_with("VmRSS:") {
                    if let Some(kb_str) = line.split_whitespace().nth(1) {
                        if let Ok(kb) = kb_str.parse::<u64>() {
                            return kb * 1024; // Convert KB to bytes
                        }
                    }
                }
            }
        }
    }

    // Fallback: return 0 as we can't accurately measure memory usage
    // In a real implementation, you would use a crate like `memory-stats`
    // or platform-specific APIs
    0
}

/// Extension trait to convert parse errors to diagnostics
trait ParseErrorExt {
    fn from_parse_error(error: crate::ParseError, file_path: &Path) -> Diagnostic;
}

impl ParseErrorExt for Diagnostic {
    fn from_parse_error(error: crate::ParseError, file_path: &Path) -> Diagnostic {
        Diagnostic {
            rule_id: "parse-error".to_string(),
            severity: crate::Severity::Error,
            message: error.message,
            location: crate::Location {
                file: file_path.to_path_buf(),
                line: error.line,
                column: error.column,
                end_line: None,
                end_column: None,
                offset: error.offset,
                length: error.length,
                span: Some((error.offset, error.offset + error.length)),
            },
            suggestions: vec![],
            code_snippet: None,
            code: None,
            source: Some("parser".to_string()),
            category: Some(crate::DiagnosticCategory::Correctness),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_info_completion_percentage() {
        let progress = ProgressInfo {
            total_files: 100,
            completed_files: 25,
            current_file: None,
            elapsed: Duration::from_secs(10),
            estimated_remaining: None,
        };

        assert_eq!(progress.completion_percentage(), 0.25);
    }

    #[test]
    fn test_progress_info_completion_percentage_zero_total() {
        let progress = ProgressInfo {
            total_files: 0,
            completed_files: 0,
            current_file: None,
            elapsed: Duration::from_secs(0),
            estimated_remaining: None,
        };

        assert_eq!(progress.completion_percentage(), 1.0);
    }

    #[test]
    fn test_execution_context_builder() {
        let config = FshLintConfiguration::default();
        let rules = vec![];

        let context = ExecutionContext::new(config, rules)
            .with_thread_pool_size(4)
            .with_memory_limit(1024 * 1024 * 1024); // 1GB

        assert_eq!(context.thread_pool_size, Some(4));
        assert_eq!(context.memory_limit, Some(1024 * 1024 * 1024));
    }
}
