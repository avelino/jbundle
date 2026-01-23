use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "clj-pack",
    version,
    about = "Package Clojure apps into self-contained binaries"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Build a self-contained binary from a Clojure project or JAR
    Build {
        /// Path to Clojure project directory or pre-built JAR file
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
        #[arg(long)]
        shrink: bool,
    },

    /// Clean the clj-pack cache
    Clean,

    /// Show cache and configuration info
    Info,
}
