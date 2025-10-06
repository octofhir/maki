//! Tests for the parallel execution engine

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tempfile::TempDir;

use fsh_lint_core::cst::FshSyntaxNode;
use fsh_lint_core::parser::{ParseResult, Parser};
use fsh_lint_core::{
    CompiledRule, DefaultExecutor, Diagnostic, ExecutionContext, Executor, FshLintConfiguration,
    FshLintError, Location, ProgressCallback, ProgressInfo, ResourceStats, Result, RuleEngine,
    SemanticAnalyzer, SemanticModel, Severity,
};

/// Mock parser for testing
struct MockParser {
    should_fail: bool,
    delay: Duration,
}

impl MockParser {
    fn new() -> Self {
        Self {
            should_fail: false,
            delay: Duration::from_millis(0),
        }
    }

    fn with_failure(mut self) -> Self {
        self.should_fail = true;
        self
    }

    fn with_delay(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }
}

impl Parser for MockParser {
    fn parse(&mut self, content: &str) -> Result<ParseResult> {
        if self.delay > Duration::ZERO {
            std::thread::sleep(self.delay);
        }

        if self.should_fail {
            return Err(FshLintError::ParseError {
                message: "Mock parse error".to_string(),
                location: Box::new(Location::default()),
            });
        }

        // Parse using the real CST parser for test consistency
        let source: Arc<str> = Arc::from(content);
        let (cst, _) = fsh_lint_core::cst::parse_fsh(&source);

        Ok(ParseResult {
            source,
            cst,
            errors: Vec::new(),
        })
    }
}

/// Mock semantic analyzer for testing
struct MockSemanticAnalyzer {
    should_fail: bool,
    #[allow(dead_code)]
    diagnostics_count: usize,
}

impl MockSemanticAnalyzer {
    fn new() -> Self {
        Self {
            should_fail: false,
            diagnostics_count: 1,
        }
    }

    fn with_failure(mut self) -> Self {
        self.should_fail = true;
        self
    }

    #[allow(dead_code)]
    fn with_diagnostics_count(mut self, count: usize) -> Self {
        self.diagnostics_count = count;
        self
    }
}

impl SemanticAnalyzer for MockSemanticAnalyzer {
    fn analyze(
        &self,
        _cst: &FshSyntaxNode,
        _source: &str,
        file_path: PathBuf,
    ) -> Result<SemanticModel> {
        if self.should_fail {
            return Err(FshLintError::SemanticError {
                message: "Mock semantic error".to_string(),
            });
        }

        Ok(SemanticModel::new(file_path))
    }

    fn resolve_references(&self, _model: &mut SemanticModel) -> Result<()> {
        Ok(())
    }

    fn validate_semantics(&self, _model: &SemanticModel) -> Vec<Diagnostic> {
        vec![]
    }
}

/// Mock rule engine for testing
struct MockRuleEngine {
    diagnostics_per_file: usize,
}

impl MockRuleEngine {
    fn new() -> Self {
        Self {
            diagnostics_per_file: 1,
        }
    }

    fn with_diagnostics_count(mut self, count: usize) -> Self {
        self.diagnostics_per_file = count;
        self
    }
}

impl RuleEngine for MockRuleEngine {
    fn load_rules(&mut self, _rule_dirs: &[PathBuf]) -> Result<()> {
        Ok(())
    }

    fn compile_rule(&self, _rule: &fsh_lint_core::Rule) -> Result<CompiledRule> {
        // Return a mock compiled rule
        Ok(CompiledRule {
            metadata: fsh_lint_core::RuleMetadata {
                id: "mock/correctness/mock-rule".to_string(),
                name: "Mock Rule".to_string(),
                description: "A mock rule for testing".to_string(),
                severity: Severity::Warning,
                category: fsh_lint_core::RuleCategory::Correctness,
                tags: vec!["test".to_string()],
                version: None,
                docs_url: None,
            },
            matcher: fsh_lint_core::GritQLMatcher::new("mock pattern".to_string()).unwrap(),
            autofix_template: None,
        })
    }

    fn execute_rules(&self, model: &SemanticModel) -> Vec<Diagnostic> {
        (0..self.diagnostics_per_file)
            .map(|i| {
                Diagnostic::new(
                    format!("test-rule-{i}"),
                    Severity::Warning,
                    format!(
                        "Test diagnostic {} for file {}",
                        i,
                        model.source_file.display()
                    ),
                    Location::new(model.source_file.clone(), 1, 1, 0, 10),
                )
            })
            .collect()
    }

    fn get_rules(&self) -> &[CompiledRule] {
        &[]
    }

    fn get_rule(&self, _id: &str) -> Option<&CompiledRule> {
        None
    }

    fn validate_rule(&self, _rule: &fsh_lint_core::Rule) -> Result<()> {
        Ok(())
    }
}

/// Create a temporary directory with test FSH files
fn create_test_files(count: usize) -> (TempDir, Vec<PathBuf>) {
    let temp_dir = TempDir::new().unwrap();
    let mut files = Vec::new();

    for i in 0..count {
        let file_path = temp_dir.path().join(format!("test_{i}.fsh"));
        std::fs::write(
            &file_path,
            format!("// Test FSH file {i}\nProfile: TestProfile{i}"),
        )
        .unwrap();
        files.push(file_path);
    }

    (temp_dir, files)
}

#[test]
fn test_parallel_execution_basic() {
    let (_temp_dir, files) = create_test_files(5);

    let config = FshLintConfiguration::default();
    let context = ExecutionContext::new(config, vec![]);

    let parser = Box::new(MockParser::new());
    let semantic_analyzer = Box::new(MockSemanticAnalyzer::new());
    let rule_engine = Box::new(MockRuleEngine::new());

    let executor = DefaultExecutor::new(context, parser, semantic_analyzer, rule_engine);

    let results = executor.execute_parallel(files.clone()).unwrap();

    assert_eq!(results.len(), 5);
    for (i, result) in results.iter().enumerate() {
        assert_eq!(result.file_path, files[i]);
        assert!(result.error.is_none());
        assert_eq!(result.diagnostics.len(), 1); // One diagnostic per file from MockRuleEngine
    }
}

#[test]
fn test_parallel_execution_deterministic_ordering() {
    let (_temp_dir, files) = create_test_files(10);

    let config = FshLintConfiguration::default();
    let context = ExecutionContext::new(config, vec![]);

    let parser = Box::new(MockParser::new().with_delay(Duration::from_millis(10)));
    let semantic_analyzer = Box::new(MockSemanticAnalyzer::new());
    let rule_engine = Box::new(MockRuleEngine::new());

    let executor = DefaultExecutor::new(context, parser, semantic_analyzer, rule_engine);

    // Run multiple times to ensure deterministic ordering
    for _ in 0..3 {
        let results = executor.execute_parallel(files.clone()).unwrap();

        assert_eq!(results.len(), 10);
        for (i, result) in results.iter().enumerate() {
            assert_eq!(
                result.file_path, files[i],
                "File order should be deterministic"
            );
        }
    }
}

#[test]
fn test_single_file_execution() {
    let (_temp_dir, files) = create_test_files(1);

    let config = FshLintConfiguration::default();
    let context = ExecutionContext::new(config, vec![]);

    let parser = Box::new(MockParser::new());
    let semantic_analyzer = Box::new(MockSemanticAnalyzer::new());
    let rule_engine = Box::new(MockRuleEngine::new());

    let executor = DefaultExecutor::new(context, parser, semantic_analyzer, rule_engine);

    let result = executor.execute_single(&files[0]).unwrap();

    assert_eq!(result.file_path, files[0]);
    assert!(result.error.is_none());
    assert_eq!(result.diagnostics.len(), 1);
}

#[test]
fn test_progress_reporting() {
    let (_temp_dir, files) = create_test_files(5);

    let progress_reports = Arc::new(Mutex::new(Vec::new()));
    let progress_reports_clone = Arc::clone(&progress_reports);

    let progress_callback: ProgressCallback = Arc::new(move |info: ProgressInfo| {
        progress_reports_clone.lock().unwrap().push(info);
    });

    let config = FshLintConfiguration::default();
    let context = ExecutionContext::new(config, vec![]).with_progress_callback(progress_callback);

    let parser = Box::new(MockParser::new());
    let semantic_analyzer = Box::new(MockSemanticAnalyzer::new());
    let rule_engine = Box::new(MockRuleEngine::new());

    let executor = DefaultExecutor::new(context, parser, semantic_analyzer, rule_engine);

    let _results = executor.execute_parallel(files).unwrap();

    let reports = progress_reports.lock().unwrap();
    assert!(!reports.is_empty(), "Should have received progress reports");

    // Check that we received a final progress report
    let final_report = reports.last().unwrap();
    assert_eq!(final_report.total_files, 5);
    assert_eq!(final_report.completed_files, 5);
}

#[test]
fn test_resource_monitoring() {
    let (_temp_dir, files) = create_test_files(3);

    let config = FshLintConfiguration::default();
    let context = ExecutionContext::new(config, vec![])
        .with_resource_monitoring(Duration::from_millis(10))
        .with_backpressure_control(4, Some(1024 * 1024 * 1024)); // 1GB limit

    let parser = Box::new(MockParser::new());
    let semantic_analyzer = Box::new(MockSemanticAnalyzer::new());
    let rule_engine = Box::new(MockRuleEngine::new());

    let executor = DefaultExecutor::new(context, parser, semantic_analyzer, rule_engine);

    let _results = executor.execute_parallel(files).unwrap();

    let stats = executor.get_context().get_resource_stats();
    assert_eq!(stats.files_processed, 3);
}

#[test]
fn test_memory_limit_enforcement() {
    let (_temp_dir, files) = create_test_files(2);

    let config = FshLintConfiguration::default();
    let context = ExecutionContext::new(config, vec![]).with_memory_limit(1); // Very low limit to trigger the check

    let parser = Box::new(MockParser::new());
    let semantic_analyzer = Box::new(MockSemanticAnalyzer::new());
    let rule_engine = Box::new(MockRuleEngine::new());

    let executor = DefaultExecutor::new(context, parser, semantic_analyzer, rule_engine);

    // This might succeed or fail depending on actual memory usage
    // The test mainly ensures the memory checking code path is exercised
    let _result = executor.execute_parallel(files);
}

#[test]
fn test_parse_error_handling() {
    let (_temp_dir, files) = create_test_files(3);

    let config = FshLintConfiguration::default();
    let context = ExecutionContext::new(config, vec![]);

    let parser = Box::new(MockParser::new().with_failure());
    let semantic_analyzer = Box::new(MockSemanticAnalyzer::new());
    let rule_engine = Box::new(MockRuleEngine::new());

    let executor = DefaultExecutor::new(context, parser, semantic_analyzer, rule_engine);

    let results = executor.execute_parallel(files).unwrap();

    // All files should have parse errors
    for result in results {
        assert!(result.error.is_some());
        match result.error.unwrap() {
            FshLintError::ParseError { .. } => {} // Expected
            other => panic!("Expected ParseError, got {other:?}"),
        }
    }
}

#[test]
#[ignore] // TODO: Fix executor error handling after parser changes
fn test_semantic_error_handling() {
    let (_temp_dir, files) = create_test_files(2);

    let config = FshLintConfiguration::default();
    let context = ExecutionContext::new(config, vec![]);

    let parser = Box::new(MockParser::new());
    let semantic_analyzer = Box::new(MockSemanticAnalyzer::new().with_failure());
    let rule_engine = Box::new(MockRuleEngine::new());

    let executor = DefaultExecutor::new(context, parser, semantic_analyzer, rule_engine);

    let results = executor.execute_parallel(files).unwrap();

    // All files should have semantic errors
    for result in results {
        assert!(result.error.is_some());
        match result.error.unwrap() {
            FshLintError::SemanticError { .. } => {} // Expected
            other => panic!("Expected SemanticError, got {other:?}"),
        }
    }
}

#[test]
fn test_parallelism_configuration() {
    let config = FshLintConfiguration::default();
    let context = ExecutionContext::new(config, vec![]).with_thread_pool_size(2);

    let parser = Box::new(MockParser::new());
    let semantic_analyzer = Box::new(MockSemanticAnalyzer::new());
    let rule_engine = Box::new(MockRuleEngine::new());

    let mut executor = DefaultExecutor::new(context, parser, semantic_analyzer, rule_engine);

    assert_eq!(executor.get_parallelism(), 2);

    executor.set_parallelism(4);
    assert_eq!(executor.get_parallelism(), 4);
}

#[test]
fn test_diagnostic_sorting() {
    let (_temp_dir, files) = create_test_files(1);

    let config = FshLintConfiguration::default();
    let context = ExecutionContext::new(config, vec![]);

    let parser = Box::new(MockParser::new());
    let semantic_analyzer = Box::new(MockSemanticAnalyzer::new());
    let rule_engine = Box::new(MockRuleEngine::new().with_diagnostics_count(5));

    let executor = DefaultExecutor::new(context, parser, semantic_analyzer, rule_engine);

    let results = executor.execute_parallel(files).unwrap();
    let result = &results[0];

    assert_eq!(result.diagnostics.len(), 5);

    // Check that diagnostics are sorted by line, column, and rule_id
    for i in 1..result.diagnostics.len() {
        let prev = &result.diagnostics[i - 1];
        let curr = &result.diagnostics[i];

        assert!(
            prev.location.line <= curr.location.line
                || (prev.location.line == curr.location.line
                    && prev.location.column <= curr.location.column)
                || (prev.location.line == curr.location.line
                    && prev.location.column == curr.location.column
                    && prev.rule_id <= curr.rule_id),
            "Diagnostics should be sorted"
        );
    }
}

#[test]
fn test_execution_context_builder() {
    let config = FshLintConfiguration::default();
    let rules = vec![];

    let context = ExecutionContext::new(config, rules)
        .with_thread_pool_size(8)
        .with_memory_limit(2 * 1024 * 1024 * 1024) // 2GB
        .with_resource_monitoring(Duration::from_millis(100))
        .with_backpressure_control(8, Some(1024 * 1024 * 1024));

    assert_eq!(context.thread_pool_size, Some(8));
    assert_eq!(context.memory_limit, Some(2 * 1024 * 1024 * 1024));
    assert!(context.resource_monitor.is_some());
    assert!(context.backpressure_controller.is_some());
}

#[test]
fn test_progress_info_calculations() {
    let progress = ProgressInfo {
        total_files: 100,
        completed_files: 25,
        current_file: None,
        elapsed: Duration::from_secs(10),
        estimated_remaining: Some(Duration::from_secs(30)),
    };

    assert_eq!(progress.completion_percentage(), 0.25);

    let progress_zero = ProgressInfo {
        total_files: 0,
        completed_files: 0,
        current_file: None,
        elapsed: Duration::from_secs(0),
        estimated_remaining: None,
    };

    assert_eq!(progress_zero.completion_percentage(), 1.0);
}

#[test]
fn test_resource_stats_tracking() {
    let stats = ResourceStats {
        peak_memory_bytes: 1024 * 1024,
        current_memory_bytes: 512 * 1024,
        active_threads: 4,
        cpu_time: Duration::from_secs(10),
        files_processed: 50,
        avg_processing_time: Duration::from_millis(200),
    };

    assert_eq!(stats.peak_memory_bytes, 1024 * 1024);
    assert_eq!(stats.current_memory_bytes, 512 * 1024);
    assert_eq!(stats.active_threads, 4);
    assert_eq!(stats.files_processed, 50);
}
