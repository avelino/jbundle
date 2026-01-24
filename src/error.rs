use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum PackError {
    #[error(
        "no build system found in {0} (expected deps.edn, project.clj, pom.xml, build.gradle, or build.gradle.kts)"
    )]
    NoBuildSystem(PathBuf),

    #[error("build failed: {0}")]
    BuildFailed(String),

    #[error("uberjar not found at {0}")]
    UberjarNotFound(PathBuf),

    #[error("JDK download failed: {0}")]
    JdkDownload(String),

    #[error("SHA256 mismatch: expected {expected}, got {actual}")]
    ChecksumMismatch { expected: String, actual: String },

    #[error("jdeps failed: {0}")]
    JdepsFailed(String),

    #[error("jlink failed: {0}")]
    JlinkFailed(String),

    #[error("cache lock timeout: another process is downloading JDK {version} for {target}")]
    CacheLockTimeout { version: u8, target: String },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("ZIP error: {0}")]
    Zip(#[from] zip::result::ZipError),

    #[error("shrink failed: {0}")]
    ShrinkFailed(String),

    #[error("CRaC is not supported by this JDK")]
    CracNotSupported,

    #[error("CRaC checkpoint failed: {0}")]
    CracCheckpointFailed(String),

    #[error("invalid JVM profile: {0} (expected: cli, server)")]
    InvalidProfile(String),

    #[error(
        "project requires Java {required}+ but --java-version is {configured}\n  \
         Detected: class file version {class_version} (Java {required}) in {class_file}\n  \
         Fix: use --java-version {required} or higher"
    )]
    JavaVersionMismatch {
        required: u8,
        configured: u8,
        class_version: u16,
        class_file: String,
    },
}
