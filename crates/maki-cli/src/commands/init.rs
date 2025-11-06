//! Init command implementation
//!
//! Creates complete, buildable FHIR Implementation Guide projects with
//! the unified MAKI configuration format.

use colored::Colorize;
use inquire::{Select, Text};
use maki_core::Result;
use maki_core::config::{
    FilesConfiguration, FormatterConfiguration, LinterConfiguration, SushiConfiguration,
    UnifiedConfig,
};
use std::fs;
use std::path::Path;
use tracing::{info, warn};

/// Project metadata collected from user
#[derive(Debug, Clone)]
struct ProjectMetadata {
    name: String,
    id: String,
    canonical: String,
    fhir_version: String,
    status: String,
    version: String,
    publisher_name: String,
    #[allow(dead_code)]
    publisher_url: String,
}

impl ProjectMetadata {
    fn default_with_name(name: String) -> Self {
        Self {
            id: format!("my.example.{}", name.to_lowercase()),
            canonical: format!("http://example.org/fhir/{}", name.to_lowercase()),
            fhir_version: "4.0.1".to_string(),
            status: "draft".to_string(),
            version: "0.1.0".to_string(),
            publisher_name: "Example Publisher".to_string(),
            publisher_url: "http://example.org".to_string(),
            name,
        }
    }
}

/// Initialize a new FSH project
pub async fn init_command(name: Option<String>, default_all: bool) -> Result<()> {
    print_header();

    // 1. Collect project metadata
    let metadata = if default_all {
        let proj_name = name.unwrap_or_else(|| "MyIG".to_string());
        println!(
            "📋 Using default values for project '{}'...\n",
            proj_name.bright_cyan()
        );
        ProjectMetadata::default_with_name(proj_name)
    } else {
        collect_metadata_interactive(name)?
    };

    // 2. Check if directory exists
    if Path::new(&metadata.name).exists() {
        eprintln!(
            "\n❌ Error: Directory '{}' already exists",
            metadata.name.red()
        );
        return Err(maki_core::MakiError::config_error(format!(
            "Directory '{}' already exists",
            metadata.name
        )));
    }

    // 3. Create directory structure
    println!("\n📁 {}", "Creating project structure...".bold());
    create_project_structure(&metadata)?;
    println!("   ✓ Created directories");

    // 4. Generate files
    println!("📝 {}", "Generating configuration files...".bold());
    generate_maki_config(&metadata)?;
    println!("   ✓ Generated maki.yaml");

    generate_ig_ini(&metadata)?;
    println!("   ✓ Generated ig.ini");

    generate_gitignore(&metadata.name)?;
    println!("   ✓ Generated .gitignore");

    println!("✍️  {}", "Creating sample files...".bold());
    generate_sample_fsh(&metadata)?;
    println!("   ✓ Generated sample FSH file");

    generate_index_page(&metadata)?;
    println!("   ✓ Generated index page");

    // 5. Download publisher scripts
    println!("⬇️  {}", "Downloading IG Publisher scripts...".bold());
    match download_publisher_scripts(&metadata.name).await {
        Ok(_) => println!("   ✓ Downloaded publisher scripts"),
        Err(e) => {
            warn!("Failed to download scripts: {}", e);
            println!("   ⚠️  Could not download scripts (you can add them manually later)");
        }
    }

    // 6. Success message
    print_success_message(&metadata.name);

    Ok(())
}

fn print_header() {
    println!("\n╭───────────────────────────────────────────────────────────╮");
    println!(
        "│{}│",
        "          MAKI Project Initialization              "
            .bright_cyan()
            .bold()
    );
    println!("╰───────────────────────────────────────────────────────────╯\n");
}

fn collect_metadata_interactive(name: Option<String>) -> Result<ProjectMetadata> {
    // Project name
    let name = name.unwrap_or_else(|| {
        Text::new("Project name:")
            .with_default("MyIG")
            .prompt()
            .unwrap_or_else(|_| "MyIG".to_string())
    });

    // Project ID
    let default_id = format!("my.example.{}", name.to_lowercase().replace(' ', ""));
    let id = Text::new("Project ID (e.g., my.example.ig):")
        .with_default(&default_id)
        .prompt()
        .unwrap_or(default_id);

    // Canonical URL
    let default_canonical = format!(
        "http://example.org/fhir/{}",
        name.to_lowercase().replace(' ', "-")
    );
    let canonical = Text::new("Canonical URL:")
        .with_default(&default_canonical)
        .prompt()
        .unwrap_or(default_canonical);

    // FHIR Version
    let fhir_version = Select::new("FHIR Version:", vec!["4.0.1", "4.3.0", "5.0.0"])
        .prompt()
        .unwrap_or("4.0.1")
        .to_string();

    // Status
    let status = Select::new("Status:", vec!["draft", "active", "retired"])
        .prompt()
        .unwrap_or("draft")
        .to_string();

    // Version
    let version = Text::new("Version:")
        .with_default("0.1.0")
        .prompt()
        .unwrap_or_else(|_| "0.1.0".to_string());

    // Publisher
    let publisher_name = Text::new("Publisher name:")
        .with_default("Example Publisher")
        .prompt()
        .unwrap_or_else(|_| "Example Publisher".to_string());

    let publisher_url = Text::new("Publisher URL:")
        .with_default("http://example.org")
        .prompt()
        .unwrap_or_else(|_| "http://example.org".to_string());

    Ok(ProjectMetadata {
        name,
        id,
        canonical,
        fhir_version,
        status,
        version,
        publisher_name,
        publisher_url,
    })
}

fn create_project_structure(meta: &ProjectMetadata) -> Result<()> {
    let base = Path::new(&meta.name);

    fs::create_dir_all(base.join("input/fsh"))?;
    fs::create_dir_all(base.join("input/pagecontent"))?;

    Ok(())
}

fn generate_maki_config(meta: &ProjectMetadata) -> Result<()> {
    // Start with no dependencies - users can add them as needed
    let config = UnifiedConfig {
        schema: Some("https://octofhir.github.io/maki/schema/v1.json".to_string()),
        root: Some(true),
        dependencies: None,
        build: Some(SushiConfiguration {
            canonical: meta.canonical.clone(),
            fhir_version: vec![meta.fhir_version.clone()],
            id: Some(meta.id.clone()),
            name: Some(meta.name.clone()),
            title: Some(meta.name.clone()),
            version: Some(meta.version.clone()),
            status: Some(meta.status.clone()),
            publisher: Some(maki_core::config::PublisherInfo::String(
                meta.publisher_name.clone(),
            )),
            experimental: None,
            date: None,
            contact: None,
            description: None,
            use_context: None,
            jurisdiction: None,
            copyright: None,
            copyright_label: None,
            version_algorithm_string: None,
            version_algorithm_coding: None,
            package_id: None,
            license: None,
            dependencies: None,
            global: None,
            groups: None,
            resources: None,
            pages: None,
            index_page_content: None,
            parameters: None,
            templates: None,
            menu: None,
            fsh_only: None,
            apply_extension_metadata_to_root: None,
            instance_options: None,
            meta: None,
            implicit_rules: None,
            language: None,
            text: None,
            contained: None,
            extension: None,
            modifier_extension: None,
            url: None,
            definition: None,
        }),
        linter: Some(LinterConfiguration {
            enabled: Some(true),
            rules: Some(maki_core::config::RulesConfiguration {
                recommended: Some(true),
                ..Default::default()
            }),
            ..Default::default()
        }),
        formatter: Some(FormatterConfiguration {
            enabled: Some(true),
            indent_size: Some(2),
            line_width: Some(100),
            align_carets: Some(true),
        }),
        files: Some(FilesConfiguration {
            include: Some(vec!["input/fsh/**/*.fsh".to_string()]),
            exclude: Some(vec![
                "**/node_modules/**".to_string(),
                "**/fsh-generated/**".to_string(),
            ]),
            ..Default::default()
        }),
    };

    let yaml = serde_yaml::to_string(&config).map_err(|e| {
        maki_core::MakiError::config_error(format!("Failed to serialize config: {}", e))
    })?;

    let path = Path::new(&meta.name).join("maki.yaml");
    fs::write(path, yaml)?;

    Ok(())
}

fn generate_ig_ini(meta: &ProjectMetadata) -> Result<()> {
    let content = format!(
        r#"[IG]
ig = fsh-generated/resources/ImplementationGuide-{}.json
template = hl7.fhir.template#current
"#,
        meta.id
    );

    let path = Path::new(&meta.name).join("ig.ini");
    fs::write(path, content)?;

    Ok(())
}

fn generate_gitignore(project_name: &str) -> Result<()> {
    let content = r#"# FHIR IG Publisher outputs
fsh-generated/
output/
temp/
template/
qa/
txCache/

# OS files
.DS_Store
Thumbs.db

# Editor files
.vscode/
.idea/
*.swp
*.swo
*~

# Node modules
node_modules/

# Backups
*.backup
*.bak

# Logs
*.log
"#;

    let path = Path::new(project_name).join(".gitignore");
    fs::write(path, content)?;

    Ok(())
}

fn generate_sample_fsh(meta: &ProjectMetadata) -> Result<()> {
    let content = format!(
        r#"Profile: MyPatient
Parent: Patient
Id: my-patient
Title: "My Patient Profile"
Description: "An example patient profile for {}"
* ^version = "{}"
* ^status = #{}

* identifier 1..* MS
* identifier ^short = "Patient identifier"
* identifier ^definition = "A unique identifier for this patient"

* name 1..* MS
* name ^short = "Patient name"
* name ^definition = "The name(s) of the patient"

* birthDate 0..1 MS
* birthDate ^short = "Date of birth"
"#,
        meta.name, meta.version, meta.status
    );

    let path = Path::new(&meta.name).join("input/fsh/patient.fsh");
    fs::write(path, content)?;

    Ok(())
}

fn generate_index_page(meta: &ProjectMetadata) -> Result<()> {
    let content = format!(
        r#"# {}

## Introduction

Welcome to the {} Implementation Guide.

## Overview

This IG provides...

## Contents

- [Profiles](profiles.html)
- [Extensions](extensions.html)
- [ValueSets](valuesets.html)

## Authors

- {}
"#,
        meta.name, meta.name, meta.publisher_name
    );

    let path = Path::new(&meta.name).join("input/pagecontent/index.md");
    fs::write(path, content)?;

    Ok(())
}

async fn download_publisher_scripts(project_name: &str) -> Result<()> {
    const SCRIPTS: &[(&str, &str)] = &[
        (
            "_genonce.sh",
            "https://raw.githubusercontent.com/HL7/ig-publisher-scripts/main/_genonce.sh",
        ),
        (
            "_genonce.bat",
            "https://raw.githubusercontent.com/HL7/ig-publisher-scripts/main/_genonce.bat",
        ),
        (
            "_updatePublisher.sh",
            "https://raw.githubusercontent.com/HL7/ig-publisher-scripts/main/_updatePublisher.sh",
        ),
        (
            "_updatePublisher.bat",
            "https://raw.githubusercontent.com/HL7/ig-publisher-scripts/main/_updatePublisher.bat",
        ),
    ];

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| {
            maki_core::MakiError::config_error(format!("Failed to build HTTP client: {}", e))
        })?;

    for (filename, url) in SCRIPTS {
        info!("Downloading {}...", filename);
        let response = client
            .get(*url)
            .send()
            .await
            .map_err(|e| maki_core::MakiError::config_error(format!("HTTP error: {}", e)))?;

        let content = response.text().await.map_err(|e| {
            maki_core::MakiError::config_error(format!("Failed to read response: {}", e))
        })?;

        let path = Path::new(project_name).join(filename);
        fs::write(&path, content)?;

        // Set executable on Unix
        #[cfg(unix)]
        if filename.ends_with(".sh") {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&path)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&path, perms)?;
        }
    }

    Ok(())
}

fn print_success_message(project_name: &str) {
    println!("\n╭───────────────────────────────────────────────────────────╮");
    println!(
        "│ {}  │",
        format!("✅ Project initialized at: ./{}", project_name)
            .green()
            .bold()
    );
    println!("├───────────────────────────────────────────────────────────┤");
    println!(
        "│ {}                                             │",
        "Now try this:".bold()
    );
    println!("│                                                           │");
    println!(
        "│ > {}                                            │",
        format!("cd {}", project_name).bright_blue()
    );
    println!(
        "│ > {}                                           │",
        "maki build".bright_blue()
    );
    println!("│                                                           │");
    println!(
        "│ For guidance see: {} │",
        "https://octofhir.github.io/maki/".bright_blue().underline()
    );
    println!("╰───────────────────────────────────────────────────────────╯\n");
}
