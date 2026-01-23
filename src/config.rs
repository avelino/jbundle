use std::path::PathBuf;

use crate::error::PackError;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BuildSystem {
    DepsEdn,
    Leiningen,
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
    pub target: Target,
    pub jvm_args: Vec<String>,
    pub shrink: bool,
}

impl BuildConfig {
    pub fn cache_dir() -> Result<PathBuf, PackError> {
        let home = dirs::home_dir().ok_or_else(|| {
            PackError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "cannot determine home directory",
            ))
        })?;
        Ok(home.join(".clj-pack").join("cache"))
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
        assert!(cache.ends_with(".clj-pack/cache"));
    }
}
