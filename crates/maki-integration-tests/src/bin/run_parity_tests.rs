//! SUSHI Parity Test Runner
//!
//! Runs maki against SUSHI's test suite and generates a compatibility report.

use clap::Parser;
use maki_integration_tests::ParityTestRunner;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "run-parity-tests")]
#[command(about = "Run SUSHI parity tests for maki")]
struct Cli {
    /// Path to SUSHI executable
    #[arg(long, env = "SUSHI_PATH", default_value = "sushi")]
    sushi: PathBuf,

    /// Path to Maki executable
    #[arg(long, env = "MAKI_PATH", default_value = "./target/release/maki")]
    maki: PathBuf,

    /// Path to SUSHI test fixtures directory
    #[arg(
        long,
        env = "SUSHI_FIXTURES",
        default_value = "/Users/alexanderstreltsov/work/octofhir/sushi/test/ig/fixtures"
    )]
    fixtures: PathBuf,

    /// Output directory for reports
    #[arg(short, long, default_value = "./parity-reports")]
    output: PathBuf,

    /// Only run tests matching this pattern
    #[arg(long)]
    filter: Option<String>,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    println!("🧪 MAKI SUSHI Parity Test Suite");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    println!("📦 SUSHI: {}", cli.sushi.display());
    println!("🦀 Maki:  {}", cli.maki.display());
    println!("📂 Fixtures: {}", cli.fixtures.display());
    println!();

    // Verify paths exist
    if !cli.maki.exists() {
        eprintln!("❌ Maki executable not found: {}", cli.maki.display());
        eprintln!("   Build it with: cargo build --release --package maki-cli");
        std::process::exit(1);
    }

    if !cli.fixtures.exists() {
        eprintln!("❌ SUSHI fixtures directory not found: {}", cli.fixtures.display());
        eprintln!("   Clone SUSHI repository and update the path");
        std::process::exit(1);
    }

    println!("🔄 Running parity tests...");
    println!();

    // Create test runner
    let runner = ParityTestRunner::new(cli.sushi, cli.maki, cli.fixtures);

    // Run tests
    let report = runner.run_all()?;

    println!();
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 RESULTS");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    println!("  Total Tests:     {}", report.total_tests);
    println!("  ✅ Passed:        {}", report.passed_tests);
    println!("  ❌ Failed:        {}", report.failed_tests);
    println!("  📈 Compatibility: {:.2}%", report.compatibility_percent);
    println!();

    // Save reports
    report.save(&cli.output)?;
    println!("📄 Reports saved to: {}", cli.output.display());
    println!("   - parity_report.json");
    println!("   - parity_report.md");
    println!();

    // Print summary of failures
    if report.failed_tests > 0 {
        println!("⚠️  Failed Tests:");
        for result in &report.test_results {
            if !result.passed {
                println!("   - {}", result.test_name);
                if cli.verbose {
                    for diff in &result.differences {
                        println!("     • {}", diff);
                    }
                }
            }
        }
        println!();
    }

    // Exit with appropriate code
    if report.compatibility_percent >= 95.0 {
        println!("🎉 SUCCESS: Compatibility >= 95%");
        Ok(())
    } else {
        eprintln!("⚠️  WARNING: Compatibility < 95%");
        std::process::exit(1)
    }
}
