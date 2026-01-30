mod build;
mod cli;
mod config;
mod crac;
mod detect;
mod diagnostic;
mod error;
mod jlink;
mod jvm;
mod pack;
mod progress;
mod project_config;
mod shrink;
mod validate;

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;
use indicatif::HumanBytes;

use cli::{Cli, Command};
use config::{BuildConfig, JvmProfile, Target};
use progress::Pipeline;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Extract verbose flag before initializing tracing
    let verbose = matches!(&cli.command, Command::Build { verbose: true, .. });

    let default_level = if verbose {
        "jbundle=info"
    } else {
        "jbundle=warn"
    };
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(default_level.parse().unwrap()),
        )
        .with_target(false)
        .without_time()
        .init();

    match cli.command {
        Command::Build {
            input,
            output,
            java_version,
            target,
            jvm_args,
            shrink,
            profile,
            no_appcds,
            crac,
            verbose: _,
            compact_banner,
        } => {
            let input_path =
                std::fs::canonicalize(&input).unwrap_or_else(|_| PathBuf::from(&input));

            let project_dir = if input_path.is_dir() {
                input_path.clone()
            } else {
                std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
            };

            let project_config = project_config::load_project_config(&project_dir)?;

            let target = match target {
                Some(t) => Target::from_str(&t).context(format!(
                    "invalid target: {t}. Use: linux-x64, linux-aarch64, macos-x64, macos-aarch64"
                ))?,
                None => match project_config.as_ref().and_then(|c| c.target.as_deref()) {
                    Some(t) => Target::from_str(t).context(format!(
                        "invalid target in jbundle.toml: {t}. Use: linux-x64, linux-aarch64, macos-x64, macos-aarch64"
                    ))?,
                    None => Target::current(),
                },
            };

            let java_version_explicit = java_version.is_some()
                || project_config
                    .as_ref()
                    .and_then(|c| c.java_version)
                    .is_some();
            let java_version = java_version
                .or(project_config.as_ref().and_then(|c| c.java_version))
                .unwrap_or(21);

            let jvm_args = if jvm_args.is_empty() {
                project_config
                    .as_ref()
                    .and_then(|c| c.jvm_args.clone())
                    .unwrap_or_default()
            } else {
                jvm_args
            };

            let shrink = shrink
                || project_config
                    .as_ref()
                    .and_then(|c| c.shrink)
                    .unwrap_or(false);

            let profile_str = profile
                .or_else(|| project_config.as_ref().and_then(|c| c.profile.clone()))
                .unwrap_or_else(|| "server".to_string());
            let jvm_profile = JvmProfile::from_str(&profile_str)
                .context(format!("invalid profile: {profile_str}"))?;

            let appcds = if no_appcds {
                false
            } else {
                project_config
                    .as_ref()
                    .and_then(|c| c.appcds)
                    .unwrap_or(true)
            };

            let crac = crac
                || project_config
                    .as_ref()
                    .and_then(|c| c.crac)
                    .unwrap_or(false);

            let compact_banner = compact_banner
                || project_config
                    .as_ref()
                    .and_then(|c| c.compact_banner)
                    .unwrap_or(false);

            let config = BuildConfig {
                input: input_path,
                output: PathBuf::from(&output),
                java_version,
                java_version_explicit,
                target,
                jvm_args,
                shrink,
                profile: jvm_profile,
                appcds,
                crac,
                compact_banner,
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

fn calculate_steps(is_jar_input: bool, shrink: bool, crac: bool) -> usize {
    let base = if is_jar_input { 1 } else { 2 }; // JAR or detect+build
    let shrink_step = if shrink { 1 } else { 0 };
    let crac_step = if crac { 1 } else { 0 };
    base + shrink_step + 4 + crac_step // +4 = JDK, jdeps, jlink, pack
}

async fn run_build(config: BuildConfig) -> Result<()> {
    let is_jar_input = config.input.extension().is_some_and(|e| e == "jar");
    let total_steps = calculate_steps(is_jar_input, config.shrink, config.crac);
    let mut pipeline = Pipeline::new(total_steps);

    eprintln!();

    // Step: Detect build system (only for project directories)
    let jar_path = if is_jar_input {
        let step = pipeline.start_step("Using pre-built JAR");
        let jar = config.input.clone();
        Pipeline::finish_step(&step, &format!("JAR: {}", jar.display()));
        jar
    } else {
        let step = pipeline.start_step("Detecting build system");
        let system = detect::detect_build_system(&config.input)?;
        Pipeline::finish_step(&step, &format!("{:?}", system));

        let build_desc = build::build_command_description(system);
        let step = pipeline.start_step(&format!("Building uberjar ({})", build_desc));
        let jar = build::build_uberjar(&config.input, system)?;
        Pipeline::finish_step(
            &step,
            &format!("{}", jar.file_name().unwrap_or_default().to_string_lossy()),
        );
        jar
    };

    // Step: Shrink JAR (optional)
    let jar_path = if config.shrink {
        let step = pipeline.start_step("Shrinking JAR");
        let result = shrink::shrink_jar(&jar_path)?;
        if result.shrunk_size < result.original_size {
            let reduction = result.original_size - result.shrunk_size;
            let pct = (reduction as f64 / result.original_size as f64) * 100.0;
            Pipeline::finish_step(
                &step,
                &format!(
                    "{} -> {} (-{:.0}%)",
                    HumanBytes(result.original_size),
                    HumanBytes(result.shrunk_size),
                    pct,
                ),
            );
        } else {
            Pipeline::finish_step(&step, "no reduction (using original)");
        }
        result.jar_path
    } else {
        jar_path
    };

    // Validate/detect Java version (no step, inline)
    let java_version = validate::resolve_java_version(
        &jar_path,
        config.java_version,
        config.java_version_explicit,
        pipeline.mp(),
    )?;

    // Step: Download/ensure JDK
    let step = pipeline.start_step(&format!("Downloading JDK {}", java_version));
    let jdk_path = jvm::ensure_jdk(java_version, &config.target, pipeline.mp()).await?;
    Pipeline::finish_step(&step, "ready");

    // Step: Detect modules (jdeps)
    let step = pipeline.start_step("Analyzing module dependencies");
    let temp_dir = tempfile::tempdir()?;
    let modules = jlink::detect_modules(&jdk_path, &jar_path)?;
    let module_count = modules.split(',').count();
    Pipeline::finish_step(&step, &format!("{} modules", module_count));

    // Step: Create minimal runtime (jlink)
    let step = pipeline.start_step("Creating minimal runtime (jlink)");
    let runtime_path = jlink::create_runtime(&jdk_path, &modules, temp_dir.path())?;
    Pipeline::finish_step(&step, "done");

    // Step: CRaC checkpoint (optional)
    let crac_path = if config.crac {
        let step = pipeline.start_step("Creating CRaC checkpoint");
        match crac::create_checkpoint(&runtime_path, &jdk_path, &jar_path, temp_dir.path()) {
            Ok(cp) => {
                let cp_size = std::fs::metadata(&cp)?.len();
                Pipeline::finish_step(&step, &format!("{} checkpoint", HumanBytes(cp_size)));
                Some(cp)
            }
            Err(e) => {
                Pipeline::finish_step(&step, &format!("skipped ({})", e));
                None
            }
        }
    } else {
        None
    };

    let compact_banner = config.compact_banner;

    // Step: Pack binary
    let step = pipeline.start_step("Packing binary");
    pack::create_binary(&pack::PackOptions {
        runtime_dir: &runtime_path,
        jar_path: &jar_path,
        crac_path: crac_path.as_deref(),
        output: &config.output,
        jvm_args: &config.jvm_args,
        profile: &config.profile,
        appcds: config.appcds,
        java_version,
        compact_banner,
    })?;
    let size = std::fs::metadata(&config.output)?.len();
    Pipeline::finish_step(
        &step,
        &format!("{} ({})", config.output.display(), HumanBytes(size)),
    );

    pipeline.finish(&config.output.display().to_string());

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
