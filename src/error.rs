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
}
