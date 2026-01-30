use std::path::PathBuf;

use crate::error::PackError;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum JvmProfile {
    Cli,
    Server,
}

/// Known JVM garbage collector flags
const GC_FLAGS: &[&str] = &[
    "-XX:+UseSerialGC",
    "-XX:+UseParallelGC",
    "-XX:+UseG1GC",
    "-XX:+UseZGC",
    "-XX:+UseShenandoahGC",
    "-XX:+UseEpsilonGC",
];

impl JvmProfile {
    pub fn flags(&self) -> Vec<&'static str> {
        match self {
            JvmProfile::Cli => vec![
                "-XX:+TieredCompilation",
                "-XX:TieredStopAtLevel=1",
                "-XX:+UseSerialGC",
            ],
            JvmProfile::Server => vec![],
        }
    }

    pub fn from_str(s: &str) -> Result<Self, PackError> {
        match s {
            "cli" => Ok(JvmProfile::Cli),
            "server" => Ok(JvmProfile::Server),
            other => Err(PackError::InvalidProfile(other.to_string())),
        }
    }

    /// Returns the GC flag used by this profile, if any
    pub fn gc_flag(&self) -> Option<&'static str> {
        match self {
            JvmProfile::Cli => Some("-XX:+UseSerialGC"),
            JvmProfile::Server => None,
        }
    }

    /// Returns the profile name as a string
    pub fn name(&self) -> &'static str {
        match self {
            JvmProfile::Cli => "cli",
            JvmProfile::Server => "server",
        }
    }
}

/// Result of GC conflict detection
#[derive(Debug)]
pub struct GcConflict {
    pub profile_gc: &'static str,
    pub jvm_args_gc: String,
    pub profile_name: &'static str,
}

/// Check for GC conflicts between profile and jvm_args
pub fn detect_gc_conflict(profile: &JvmProfile, jvm_args: &[String]) -> Option<GcConflict> {
    let profile_gc = profile.gc_flag()?;

    // Find any GC flag in jvm_args that conflicts with profile
    for arg in jvm_args {
        for &gc_flag in GC_FLAGS {
            if arg == gc_flag && gc_flag != profile_gc {
                return Some(GcConflict {
                    profile_gc,
                    jvm_args_gc: arg.clone(),
                    profile_name: profile.name(),
                });
            }
        }
    }

    None
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BuildSystem {
    DepsEdn,
    Leiningen,
    Maven,
    Gradle,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TargetOs {
    Linux,
    MacOs,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TargetArch {
    X86_64,
    Aarch64,
}

#[derive(Debug, Clone)]
pub struct Target {
    pub os: TargetOs,
    pub arch: TargetArch,
}

impl Target {
    pub fn current() -> Self {
        let os = if cfg!(target_os = "macos") {
            TargetOs::MacOs
        } else {
            TargetOs::Linux
        };
        let arch = if cfg!(target_arch = "aarch64") {
            TargetArch::Aarch64
        } else {
            TargetArch::X86_64
        };
        Self { os, arch }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "linux-x64" => Some(Self {
                os: TargetOs::Linux,
                arch: TargetArch::X86_64,
            }),
            "linux-aarch64" => Some(Self {
                os: TargetOs::Linux,
                arch: TargetArch::Aarch64,
            }),
            "macos-x64" => Some(Self {
                os: TargetOs::MacOs,
                arch: TargetArch::X86_64,
            }),
            "macos-aarch64" => Some(Self {
                os: TargetOs::MacOs,
                arch: TargetArch::Aarch64,
            }),
            _ => None,
        }
    }

    pub fn adoptium_os(&self) -> &'static str {
        match self.os {
            TargetOs::Linux => "linux",
            TargetOs::MacOs => "mac",
        }
    }

    pub fn adoptium_arch(&self) -> &'static str {
        match self.arch {
            TargetArch::X86_64 => "x64",
            TargetArch::Aarch64 => "aarch64",
        }
    }
}

#[derive(Debug, Clone)]
pub struct BuildConfig {
    pub input: PathBuf,
    pub output: PathBuf,
    pub java_version: u8,
    pub java_version_explicit: bool,
    pub target: Target,
    pub jvm_args: Vec<String>,
    pub shrink: bool,
    pub profile: JvmProfile,
    pub appcds: bool,
    pub crac: bool,
    /// Gradle subproject to build (for multi-project builds)
    pub gradle_project: Option<String>,
    /// Build all application subprojects (Gradle multi-project)
    pub build_all: bool,
    /// Manual module override (bypasses jdeps detection)
    pub modules_override: Option<Vec<String>>,
    /// Path to existing jlink runtime to reuse
    pub jlink_runtime: Option<PathBuf>,
}

impl BuildConfig {
    pub fn cache_dir() -> Result<PathBuf, PackError> {
        let home = dirs::home_dir().ok_or_else(|| {
            PackError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "cannot determine home directory",
            ))
        })?;
        Ok(home.join(".jbundle").join("cache"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_from_str_valid() {
        let t = Target::from_str("linux-x64").unwrap();
        assert_eq!(t.os, TargetOs::Linux);
        assert_eq!(t.arch, TargetArch::X86_64);

        let t = Target::from_str("linux-aarch64").unwrap();
        assert_eq!(t.os, TargetOs::Linux);
        assert_eq!(t.arch, TargetArch::Aarch64);

        let t = Target::from_str("macos-x64").unwrap();
        assert_eq!(t.os, TargetOs::MacOs);
        assert_eq!(t.arch, TargetArch::X86_64);

        let t = Target::from_str("macos-aarch64").unwrap();
        assert_eq!(t.os, TargetOs::MacOs);
        assert_eq!(t.arch, TargetArch::Aarch64);
    }

    #[test]
    fn target_from_str_invalid() {
        assert!(Target::from_str("windows-x64").is_none());
        assert!(Target::from_str("").is_none());
        assert!(Target::from_str("linux").is_none());
    }

    #[test]
    fn adoptium_os_mapping() {
        let linux = Target {
            os: TargetOs::Linux,
            arch: TargetArch::X86_64,
        };
        assert_eq!(linux.adoptium_os(), "linux");

        let macos = Target {
            os: TargetOs::MacOs,
            arch: TargetArch::X86_64,
        };
        assert_eq!(macos.adoptium_os(), "mac");
    }

    #[test]
    fn adoptium_arch_mapping() {
        let x64 = Target {
            os: TargetOs::Linux,
            arch: TargetArch::X86_64,
        };
        assert_eq!(x64.adoptium_arch(), "x64");

        let arm = Target {
            os: TargetOs::Linux,
            arch: TargetArch::Aarch64,
        };
        assert_eq!(arm.adoptium_arch(), "aarch64");
    }

    #[test]
    fn cache_dir_ends_with_expected_path() {
        let cache = BuildConfig::cache_dir().unwrap();
        assert!(cache.ends_with(".jbundle/cache"));
    }

    #[test]
    fn jvm_profile_cli_flags() {
        let flags = JvmProfile::Cli.flags();
        assert!(flags.contains(&"-XX:+TieredCompilation"));
        assert!(flags.contains(&"-XX:TieredStopAtLevel=1"));
        assert!(flags.contains(&"-XX:+UseSerialGC"));
    }

    #[test]
    fn jvm_profile_server_flags_empty() {
        let flags = JvmProfile::Server.flags();
        assert!(flags.is_empty());
    }

    #[test]
    fn jvm_profile_from_str_valid() {
        assert_eq!(JvmProfile::from_str("cli").unwrap(), JvmProfile::Cli);
        assert_eq!(JvmProfile::from_str("server").unwrap(), JvmProfile::Server);
    }

    #[test]
    fn jvm_profile_from_str_invalid() {
        assert!(JvmProfile::from_str("unknown").is_err());
    }

    #[test]
    fn detect_gc_conflict_cli_with_zgc() {
        let conflict = detect_gc_conflict(
            &JvmProfile::Cli,
            &["-Xmx512m".to_string(), "-XX:+UseZGC".to_string()],
        );
        assert!(conflict.is_some());
        let c = conflict.unwrap();
        assert_eq!(c.profile_gc, "-XX:+UseSerialGC");
        assert_eq!(c.jvm_args_gc, "-XX:+UseZGC");
    }

    #[test]
    fn detect_gc_conflict_cli_with_g1gc() {
        let conflict = detect_gc_conflict(&JvmProfile::Cli, &["-XX:+UseG1GC".to_string()]);
        assert!(conflict.is_some());
    }

    #[test]
    fn detect_gc_conflict_server_no_conflict() {
        let conflict = detect_gc_conflict(&JvmProfile::Server, &["-XX:+UseZGC".to_string()]);
        assert!(conflict.is_none());
    }

    #[test]
    fn detect_gc_conflict_cli_no_gc_in_args() {
        let conflict = detect_gc_conflict(
            &JvmProfile::Cli,
            &["-Xmx512m".to_string(), "-Dapp.env=prod".to_string()],
        );
        assert!(conflict.is_none());
    }

    #[test]
    fn detect_gc_conflict_cli_same_gc() {
        // Same GC as profile should not be a conflict
        let conflict = detect_gc_conflict(&JvmProfile::Cli, &["-XX:+UseSerialGC".to_string()]);
        assert!(conflict.is_none());
    }

    #[test]
    fn jvm_profile_gc_flag() {
        assert_eq!(JvmProfile::Cli.gc_flag(), Some("-XX:+UseSerialGC"));
        assert_eq!(JvmProfile::Server.gc_flag(), None);
    }
}
