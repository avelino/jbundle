use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "jbundle",
    version,
    about = "Package JVM apps into self-contained binaries"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Build a self-contained binary from a JVM project or JAR
    Build {
        /// Path to project directory or pre-built JAR file
        #[arg(short, long, default_value = ".")]
        input: PathBuf,

        /// Output binary path
        #[arg(short, long, default_value = "./dist/app")]
        output: PathBuf,

        /// Java version (11, 17, 21). Auto-detected from JAR if not specified.
        #[arg(long)]
        java_version: Option<u8>,

        /// Target platform (linux-x64, linux-aarch64, macos-x64, macos-aarch64)
        #[arg(long)]
        target: Option<String>,

        /// Extra JVM arguments passed to the application
        #[arg(long)]
        jvm_args: Vec<String>,

        /// Shrink the uberjar by removing non-essential files and recompressing
        #[arg(long, default_value_t = false, num_args = 0..=1, default_missing_value = "true")]
        shrink: bool,

        /// JVM startup profile (cli: fast startup, server: throughput optimized)
        #[arg(long)]
        profile: Option<String>,

        /// Disable AppCDS archive generation
        #[arg(long)]
        no_appcds: bool,

        /// Enable CRaC checkpoint for instant restore (Linux only)
        #[arg(long)]
        crac: bool,

        /// Gradle subproject to build (for multi-project builds)
        #[arg(long)]
        gradle_project: Option<String>,

        /// Build all application subprojects (Gradle multi-project)
        #[arg(long)]
        all: bool,

        /// Manual module list (bypasses jdeps detection, comma-separated)
        #[arg(long)]
        modules: Option<String>,

        /// Path to existing jlink runtime to reuse
        #[arg(long)]
        jlink_runtime: Option<PathBuf>,

        /// Enable verbose output (show build commands and details)
        #[arg(short, long)]
        verbose: bool,
    },

    /// Clean the jbundle cache
    Clean,

    /// Show cache and configuration info
    Info,
}
