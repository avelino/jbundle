use std::path::Path;

use crate::config::BuildSystem;
use crate::error::PackError;
use crate::gradle::{GradleProject, Subproject};

/// Result of build system detection.
#[derive(Debug)]
pub enum DetectedBuild {
    /// Simple build system (deps.edn, leiningen, maven, single-module gradle)
    Simple(BuildSystem),
    /// Gradle multi-project build with application subprojects
    GradleMultiProject {
        project: GradleProject,
        app_subprojects: Vec<Subproject>,
    },
}

/// Enhanced detection that identifies Gradle multi-project builds.
pub fn detect_build_system_enhanced(project_dir: &Path) -> Result<DetectedBuild, PackError> {
    // Clojure build systems
    if project_dir.join("deps.edn").exists() {
        return Ok(DetectedBuild::Simple(BuildSystem::DepsEdn));
    }
    if project_dir.join("project.clj").exists() {
        return Ok(DetectedBuild::Simple(BuildSystem::Leiningen));
    }
    // Java build systems
    if project_dir.join("pom.xml").exists() {
        return Ok(DetectedBuild::Simple(BuildSystem::Maven));
    }

    // Check for Gradle (with multi-project detection)
    if project_dir.join("build.gradle").exists() || project_dir.join("build.gradle.kts").exists() {
        // Try to parse as multi-project
        if let Some(gradle_project) = GradleProject::parse(project_dir) {
            let app_subprojects: Vec<_> = gradle_project
                .application_subprojects()
                .into_iter()
                .cloned()
                .collect();

            // If multi-project with application subprojects, return enhanced info
            if gradle_project.is_multi_project() && !app_subprojects.is_empty() {
                return Ok(DetectedBuild::GradleMultiProject {
                    project: gradle_project,
                    app_subprojects,
                });
            }
        }
        // Fall back to simple Gradle detection
        return Ok(DetectedBuild::Simple(BuildSystem::Gradle));
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
        let result = detect_build_system_enhanced(dir.path()).unwrap();
        assert!(matches!(
            result,
            DetectedBuild::Simple(BuildSystem::DepsEdn)
        ));
    }

    #[test]
    fn detects_leiningen() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("project.clj"), "(defproject foo)").unwrap();
        let result = detect_build_system_enhanced(dir.path()).unwrap();
        assert!(matches!(
            result,
            DetectedBuild::Simple(BuildSystem::Leiningen)
        ));
    }

    #[test]
    fn deps_edn_has_priority_over_project_clj() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("deps.edn"), "{}").unwrap();
        std::fs::write(dir.path().join("project.clj"), "(defproject foo)").unwrap();
        let result = detect_build_system_enhanced(dir.path()).unwrap();
        assert!(matches!(
            result,
            DetectedBuild::Simple(BuildSystem::DepsEdn)
        ));
    }

    #[test]
    fn detects_maven() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("pom.xml"), "<project/>").unwrap();
        let result = detect_build_system_enhanced(dir.path()).unwrap();
        assert!(matches!(result, DetectedBuild::Simple(BuildSystem::Maven)));
    }

    #[test]
    fn detects_gradle() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("build.gradle"), "apply plugin: 'java'").unwrap();
        let result = detect_build_system_enhanced(dir.path()).unwrap();
        assert!(matches!(result, DetectedBuild::Simple(BuildSystem::Gradle)));
    }

    #[test]
    fn detects_gradle_kts() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("build.gradle.kts"), "plugins { java }").unwrap();
        let result = detect_build_system_enhanced(dir.path()).unwrap();
        assert!(matches!(result, DetectedBuild::Simple(BuildSystem::Gradle)));
    }

    #[test]
    fn clojure_has_priority_over_java() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("deps.edn"), "{}").unwrap();
        std::fs::write(dir.path().join("pom.xml"), "<project/>").unwrap();
        let result = detect_build_system_enhanced(dir.path()).unwrap();
        assert!(matches!(
            result,
            DetectedBuild::Simple(BuildSystem::DepsEdn)
        ));
    }

    #[test]
    fn error_when_no_build_system() {
        let dir = tempdir().unwrap();
        let result = detect_build_system_enhanced(dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn detects_gradle_multi_project() {
        let dir = tempdir().unwrap();

        // settings.gradle.kts
        std::fs::write(
            dir.path().join("settings.gradle.kts"),
            r#"
rootProject.name = "myproject"
include("app")
"#,
        )
        .unwrap();

        // Root build.gradle.kts (no application)
        std::fs::write(dir.path().join("build.gradle.kts"), "plugins { java }").unwrap();

        // app subproject with application
        let app_dir = dir.path().join("app");
        std::fs::create_dir_all(&app_dir).unwrap();
        std::fs::write(
            app_dir.join("build.gradle.kts"),
            r#"
plugins {
    id("application")
}
application {
    mainClass.set("com.example.App")
}
"#,
        )
        .unwrap();

        let result = detect_build_system_enhanced(dir.path()).unwrap();
        match result {
            DetectedBuild::GradleMultiProject {
                app_subprojects, ..
            } => {
                assert_eq!(app_subprojects.len(), 1);
                assert_eq!(app_subprojects[0].name, "app");
            }
            _ => panic!("expected GradleMultiProject"),
        }
    }
}
