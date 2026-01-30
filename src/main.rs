mod build;
mod cli;
mod config;
mod crac;
mod detect;
mod diagnostic;
mod error;
mod gradle;
mod jlink;
mod jvm;
mod pack;
mod progress;
mod project_config;
mod shrink;
mod validate;

use std::io::Write;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;
use indicatif::HumanBytes;

use cli::{Cli, Command};
use config::{detect_gc_conflict, BuildConfig, JvmProfile, Target};
use error::PackError;
use gradle::Subproject;
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
            gradle_project,
            all,
            modules,
            jlink_runtime,
            verbose: _,
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

            // Gradle subproject selection (CLI > config file)
            let gradle_project = gradle_project.or_else(|| {
                project_config
                    .as_ref()
                    .and_then(|c| c.gradle_project.clone())
            });

            // Manual modules override (CLI > config file)
            let modules_override = modules
                .map(|m| m.split(',').map(|s| s.trim().to_string()).collect())
                .or_else(|| project_config.as_ref().and_then(|c| c.modules.clone()));

            // Jlink runtime path (CLI > config file)
            let jlink_runtime = jlink_runtime.or_else(|| {
                project_config
                    .as_ref()
                    .and_then(|c| c.jlink_runtime.as_ref())
                    .map(PathBuf::from)
            });

            // Check for GC conflicts between profile and jvm_args
            if let Some(conflict) = detect_gc_conflict(&jvm_profile, &jvm_args) {
                tracing::warn!(
                    "GC conflict: profile '{}' uses {} but jvm_args contains {}. \
                     The JVM cannot use multiple garbage collectors. \
                     Consider using profile = \"server\" or removing {} from jvm_args.",
                    conflict.profile_name,
                    conflict.profile_gc,
                    conflict.jvm_args_gc,
                    conflict.jvm_args_gc
                );
            }

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
                gradle_project,
                build_all: all,
                modules_override,
                jlink_runtime,
            };

            if config.build_all {
                run_build_all(config).await?;
            } else {
                run_build(config).await?;
            }
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

/// Select a Gradle subproject based on CLI flag or interactive prompt.
fn select_gradle_subproject<'a>(
    app_subprojects: &'a [Subproject],
    cli_selection: Option<&str>,
) -> Result<&'a Subproject, PackError> {
    // If CLI flag provided, find that subproject
    if let Some(name) = cli_selection {
        return app_subprojects
            .iter()
            .find(|s| s.name == name)
            .ok_or_else(|| PackError::GradleSubprojectNotFound(name.to_string()));
    }

    // If only one application subproject, use it automatically
    if app_subprojects.len() == 1 {
        return Ok(&app_subprojects[0]);
    }

    // If no application subprojects found
    if app_subprojects.is_empty() {
        return Err(PackError::NoApplicationSubproject);
    }

    // Multiple subprojects: prompt user for selection
    eprintln!("\nMultiple application subprojects found:");
    for (i, sub) in app_subprojects.iter().enumerate() {
        let desc = sub
            .main_class
            .as_deref()
            .unwrap_or("(no main class detected)");
        eprintln!("  [{}] {} - {}", i + 1, sub.name, desc);
    }
    eprintln!();
    eprintln!(
        "Tip: Add 'gradle_project = \"{}\"' to jbundle.toml to skip this prompt",
        app_subprojects[0].name
    );
    eprintln!();
    eprint!("Select subproject [1-{}]: ", app_subprojects.len());
    std::io::stderr().flush().ok();

    let mut input = String::new();
    std::io::stdin()
        .read_line(&mut input)
        .map_err(|e| PackError::BuildFailed(format!("failed to read input: {e}")))?;

    let choice: usize = input.trim().parse().map_err(|_| {
        PackError::MultipleApplicationSubprojects(
            app_subprojects.iter().map(|s| s.name.clone()).collect(),
        )
    })?;

    if choice == 0 || choice > app_subprojects.len() {
        return Err(PackError::MultipleApplicationSubprojects(
            app_subprojects.iter().map(|s| s.name.clone()).collect(),
        ));
    }

    Ok(&app_subprojects[choice - 1])
}

/// Build all application subprojects in a Gradle multi-project.
async fn run_build_all(config: BuildConfig) -> Result<()> {
    // Detect multi-project
    let detected = detect::detect_build_system_enhanced(&config.input)?;

    let app_subprojects = match detected {
        detect::DetectedBuild::GradleMultiProject {
            app_subprojects, ..
        } => app_subprojects,
        detect::DetectedBuild::Simple(_) => {
            anyhow::bail!(
                "--all flag requires a Gradle multi-project build. \
                 No subprojects with application plugin found."
            );
        }
    };

    if app_subprojects.is_empty() {
        anyhow::bail!("No application subprojects found in Gradle multi-project.");
    }

    eprintln!();
    eprintln!(
        "Building {} application subprojects:",
        app_subprojects.len()
    );
    for sub in &app_subprojects {
        let desc = sub
            .main_class
            .as_deref()
            .unwrap_or("(no main class detected)");
        eprintln!("  - {} ({})", sub.name, desc);
    }
    eprintln!();

    let base_output = config.output.clone();
    let mut built = Vec::new();

    for sub in &app_subprojects {
        // Create output path: base_output/subproject_name
        let output = base_output.join(&sub.name);

        // Create parent directory if needed
        if let Some(parent) = output.parent() {
            std::fs::create_dir_all(parent)?;
        }

        eprintln!("━━━ Building {} ━━━", sub.name);

        // Clone config with subproject-specific settings
        let sub_config = BuildConfig {
            gradle_project: Some(sub.name.clone()),
            output,
            build_all: false, // Prevent recursion
            ..config.clone()
        };

        run_build(sub_config).await?;
        built.push(sub.name.clone());
        eprintln!();
    }

    eprintln!("━━━ Build complete ━━━");
    eprintln!("Built {} binaries:", built.len());
    for name in &built {
        eprintln!("  - {}/{}", base_output.display(), name);
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
    let (jar_path, detected_modules) = if is_jar_input {
        let step = pipeline.start_step("Using pre-built JAR");
        let jar = config.input.clone();
        Pipeline::finish_step(&step, &format!("JAR: {}", jar.display()));
        (jar, Vec::new())
    } else {
        let step = pipeline.start_step("Detecting build system");
        let detected = detect::detect_build_system_enhanced(&config.input)?;

        match detected {
            detect::DetectedBuild::Simple(system) => {
                Pipeline::finish_step(&step, &format!("{:?}", system));

                let build_desc = build::build_command_description(system);
                let step = pipeline.start_step(&format!("Building uberjar ({})", build_desc));
                let jar = build::build_uberjar(&config.input, system)?;
                Pipeline::finish_step(
                    &step,
                    &format!("{}", jar.file_name().unwrap_or_default().to_string_lossy()),
                );
                (jar, Vec::new())
            }
            detect::DetectedBuild::GradleMultiProject {
                project,
                app_subprojects,
            } => {
                // Select subproject
                let selected =
                    select_gradle_subproject(&app_subprojects, config.gradle_project.as_deref())?;

                Pipeline::finish_step(&step, &format!("Gradle multi-project ({})", selected.name));

                // Extract modules from Gradle config
                let gradle_modules = selected.add_modules.clone();

                let build_desc = build::gradle_subproject_command_description(&selected.name);
                let step = pipeline.start_step(&format!("Building uberjar ({})", build_desc));
                let jar = build::build_gradle_subproject(&project.root, &selected.name)?;
                Pipeline::finish_step(
                    &step,
                    &format!("{}", jar.file_name().unwrap_or_default().to_string_lossy()),
                );
                (jar, gradle_modules)
            }
        }
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

    // Check for existing jlink runtime to reuse
    let existing_runtime = config.jlink_runtime.as_ref().and_then(|p| {
        if !p.exists() {
            tracing::warn!("provided jlink runtime not found: {}", p.display());
            return None;
        }
        let java_bin = p.join("bin").join("java");
        if !java_bin.exists() {
            tracing::warn!("provided jlink runtime missing bin/java: {}", p.display());
            return None;
        }
        tracing::info!("using provided jlink runtime: {}", p.display());
        Some(p.clone())
    });

    // Step: Download/ensure JDK
    let step = pipeline.start_step(&format!("Downloading JDK {}", java_version));
    let jdk_path = jvm::ensure_jdk(java_version, &config.target, pipeline.mp()).await?;
    Pipeline::finish_step(&step, "ready");

    let temp_dir = tempfile::tempdir()?;

    // Step: Detect modules (jdeps) - skip if using manual override or existing runtime
    let modules = if let Some(ref override_modules) = config.modules_override {
        // Use manual module override
        let step = pipeline.start_step("Using manual module override");
        let modules = override_modules.join(",");
        Pipeline::finish_step(&step, &format!("{} modules", override_modules.len()));
        modules
    } else {
        // Detect modules with jdeps, combining with Gradle-detected modules
        let step = pipeline.start_step("Analyzing module dependencies");
        let mut modules = jlink::detect_modules(&jdk_path, &jar_path)?;

        // Append Gradle-detected modules if any
        if !detected_modules.is_empty() {
            let extra = detected_modules.join(",");
            if !modules.is_empty() {
                modules.push(',');
            }
            modules.push_str(&extra);
            // Deduplicate
            let mut module_set: std::collections::HashSet<&str> = modules.split(',').collect();
            modules = module_set.drain().collect::<Vec<_>>().join(",");
        }

        let module_count = modules.split(',').count();
        Pipeline::finish_step(&step, &format!("{} modules", module_count));
        modules
    };

    // Step: Create minimal runtime (jlink) - skip if reusing existing runtime
    let runtime_path = if let Some(existing) = existing_runtime {
        let step = pipeline.start_step("Reusing existing jlink runtime");
        Pipeline::finish_step(&step, &format!("{}", existing.display()));
        existing
    } else {
        let step = pipeline.start_step("Creating minimal runtime (jlink)");
        let runtime = jlink::create_runtime(&jdk_path, &modules, temp_dir.path())?;
        Pipeline::finish_step(&step, "done");
        runtime
    };

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
