use std::path::Path;

use crate::config::BuildSystem;
use crate::error::PackError;

pub fn detect_build_system(project_dir: &Path) -> Result<BuildSystem, PackError> {
    // Clojure build systems
    if project_dir.join("deps.edn").exists() {
        return Ok(BuildSystem::DepsEdn);
    }
    if project_dir.join("project.clj").exists() {
        return Ok(BuildSystem::Leiningen);
    }
    // Java build systems
    if project_dir.join("pom.xml").exists() {
        return Ok(BuildSystem::Maven);
    }
    if project_dir.join("build.gradle").exists() || project_dir.join("build.gradle.kts").exists() {
        return Ok(BuildSystem::Gradle);
    }
    Err(PackError::NoBuildSystem(project_dir.to_path_buf()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn detects_deps_edn() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("deps.edn"), "{}").unwrap();
        let result = detect_build_system(dir.path()).unwrap();
        assert_eq!(result, BuildSystem::DepsEdn);
    }

    #[test]
    fn detects_leiningen() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("project.clj"), "(defproject foo)").unwrap();
        let result = detect_build_system(dir.path()).unwrap();
        assert_eq!(result, BuildSystem::Leiningen);
    }

    #[test]
    fn deps_edn_has_priority_over_project_clj() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("deps.edn"), "{}").unwrap();
        std::fs::write(dir.path().join("project.clj"), "(defproject foo)").unwrap();
        let result = detect_build_system(dir.path()).unwrap();
        assert_eq!(result, BuildSystem::DepsEdn);
    }

    #[test]
    fn detects_maven() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("pom.xml"), "<project/>").unwrap();
        let result = detect_build_system(dir.path()).unwrap();
        assert_eq!(result, BuildSystem::Maven);
    }

    #[test]
    fn detects_gradle() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("build.gradle"), "apply plugin: 'java'").unwrap();
        let result = detect_build_system(dir.path()).unwrap();
        assert_eq!(result, BuildSystem::Gradle);
    }

    #[test]
    fn detects_gradle_kts() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("build.gradle.kts"), "plugins { java }").unwrap();
        let result = detect_build_system(dir.path()).unwrap();
        assert_eq!(result, BuildSystem::Gradle);
    }

    #[test]
    fn clojure_has_priority_over_java() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("deps.edn"), "{}").unwrap();
        std::fs::write(dir.path().join("pom.xml"), "<project/>").unwrap();
        let result = detect_build_system(dir.path()).unwrap();
        assert_eq!(result, BuildSystem::DepsEdn);
    }

    #[test]
    fn error_when_no_build_system() {
        let dir = tempdir().unwrap();
        let result = detect_build_system(dir.path());
        assert!(result.is_err());
    }
}
