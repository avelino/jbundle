mod build;
mod cli;
mod config;
mod detect;
mod error;
mod jlink;
mod jvm;
mod pack;
mod shrink;
mod validate;

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;
use indicatif::{HumanBytes, MultiProgress, ProgressBar, ProgressStyle};

use cli::{Cli, Command};
use config::{BuildConfig, Target};

fn spinner(mp: &MultiProgress, msg: &str) -> ProgressBar {
    let sp = mp.add(ProgressBar::new_spinner());
    sp.set_style(
        ProgressStyle::default_spinner()
            .template("  {spinner:.cyan} {msg}")
            .expect("invalid spinner template"),
    );
    sp.set_message(msg.to_string());
    sp.enable_steady_tick(std::time::Duration::from_millis(80));
    sp
}

fn finish_spinner(sp: &ProgressBar, msg: &str) {
    sp.set_style(
        ProgressStyle::default_spinner()
            .template("  {msg}")
            .expect("invalid spinner template"),
    );
    sp.finish_with_message(format!("\x1b[32m✓\x1b[0m {msg}"));
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("jbundle=warn".parse().unwrap()),
        )
        .with_target(false)
        .without_time()
        .init();

    let cli = Cli::parse();

    match cli.command {
        Command::Build {
            input,
            output,
            java_version,
            target,
            jvm_args,
            shrink,
        } => {
            let target = match target {
                Some(t) => Target::from_str(&t).context(format!(
                    "invalid target: {t}. Use: linux-x64, linux-aarch64, macos-x64, macos-aarch64"
                ))?,
                None => Target::current(),
            };

            let java_version_explicit = java_version.is_some();
            let java_version = java_version.unwrap_or(21);

            let config = BuildConfig {
                input: std::fs::canonicalize(&input).unwrap_or_else(|_| PathBuf::from(&input)),
                output: PathBuf::from(&output),
                java_version,
                java_version_explicit,
                target,
                jvm_args,
                shrink,
            };

            run_build(config).await?;
        }
        Command::Clean => {
            run_clean()?;
        }
        Command::Info => {
            run_info()?;
        }
    }

    Ok(())
}

async fn run_build(config: BuildConfig) -> Result<()> {
    let mp = MultiProgress::new();
    eprintln!();

    // Step 1: Build uberjar
    let jar_path = if config.input.extension().is_some_and(|e| e == "jar") {
        let sp = spinner(&mp, "Using pre-built JAR");
        let jar = config.input.clone();
        finish_spinner(&sp, &format!("JAR: {}", jar.display()));
        jar
    } else {
        let sp = spinner(&mp, "Building uberjar...");
        let system = detect::detect_build_system(&config.input)?;
        let jar = build::build_uberjar(&config.input, system)?;
        finish_spinner(
            &sp,
            &format!(
                "Uberjar: {}",
                jar.file_name().unwrap_or_default().to_string_lossy()
            ),
        );
        jar
    };

    // Step 1.5: Shrink JAR (optional)
    let jar_path = if config.shrink {
        let sp = spinner(&mp, "Shrinking JAR...");
        let result = shrink::shrink_jar(&jar_path)?;
        if result.shrunk_size < result.original_size {
            let reduction = result.original_size - result.shrunk_size;
            let pct = (reduction as f64 / result.original_size as f64) * 100.0;
            finish_spinner(
                &sp,
                &format!(
                    "Shrunk: {} -> {} (-{:.0}%)",
                    HumanBytes(result.original_size),
                    HumanBytes(result.shrunk_size),
                    pct,
                ),
            );
        } else {
            finish_spinner(&sp, "Shrink: no reduction (using original JAR)");
        }
        result.jar_path
    } else {
        jar_path
    };

    // Step 1.7: Validate/detect Java version
    let java_version = validate::resolve_java_version(
        &jar_path,
        config.java_version,
        config.java_version_explicit,
        &mp,
    )?;

    // Step 2: Ensure JDK
    let sp = spinner(&mp, &format!("Ensuring JDK {}...", java_version));
    let jdk_path = jvm::ensure_jdk(java_version, &config.target, &mp).await?;
    finish_spinner(&sp, &format!("JDK {} ready", java_version));

    // Step 3: Detect modules
    let sp = spinner(&mp, "Detecting Java modules...");
    let temp_dir = tempfile::tempdir()?;
    let modules = jlink::detect_modules(&jdk_path, &jar_path)?;
    let module_count = modules.split(',').count();
    finish_spinner(&sp, &format!("{module_count} modules detected"));

    // Step 4: Create minimal runtime
    let sp = spinner(&mp, "Creating minimal JVM runtime...");
    let runtime_path = jlink::create_runtime(&jdk_path, &modules, temp_dir.path())?;
    finish_spinner(&sp, "Runtime created (jlink)");

    // Step 5: Pack binary
    let sp = spinner(&mp, "Packing binary...");
    pack::create_binary(&runtime_path, &jar_path, &config.output, &config.jvm_args)?;
    let size = std::fs::metadata(&config.output)?.len();
    finish_spinner(
        &sp,
        &format!("Packed: {} ({})", config.output.display(), HumanBytes(size)),
    );

    eprintln!(
        "\n  \x1b[1;32m✓\x1b[0m Binary ready: {}\n",
        config.output.display()
    );

    Ok(())
}

fn run_clean() -> Result<()> {
    let cache_dir = BuildConfig::cache_dir()?;
    if cache_dir.exists() {
        let size = dir_size(&cache_dir);
        std::fs::remove_dir_all(&cache_dir)?;
        eprintln!("Cleaned {} of cached data", HumanBytes(size));
    } else {
        eprintln!("Cache is already empty");
    }
    Ok(())
}

fn run_info() -> Result<()> {
    let cache_dir = BuildConfig::cache_dir()?;
    eprintln!("Cache directory: {}", cache_dir.display());

    if cache_dir.exists() {
        let size = dir_size(&cache_dir);
        eprintln!("Cache size:      {}", HumanBytes(size));

        let entries: Vec<_> = std::fs::read_dir(&cache_dir)?
            .filter_map(|e| e.ok())
            .collect();
        eprintln!("Cached items:    {}", entries.len());

        for entry in &entries {
            let name = entry.file_name();
            let entry_size = dir_size(&entry.path());
            eprintln!("  {} ({})", name.to_string_lossy(), HumanBytes(entry_size));
        }
    } else {
        eprintln!("Cache is empty");
    }

    eprintln!("\nCurrent platform: {:?}", Target::current());
    Ok(())
}

fn dir_size(path: &std::path::Path) -> u64 {
    walkdir(path)
}

fn walkdir(path: &std::path::Path) -> u64 {
    let mut size = 0;
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                size += walkdir(&p);
            } else if let Ok(meta) = p.metadata() {
                size += meta.len();
            }
        }
    }
    size
}
