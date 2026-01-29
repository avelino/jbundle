//! Parser for Gradle multi-project configurations.
//!
//! Detects subprojects with the `application` plugin and extracts
//! relevant configuration like mainClass, mainModule, and addModules.

use std::path::{Path, PathBuf};

use regex::Regex;

/// Represents a parsed Gradle project (potentially multi-module).
#[derive(Debug, Clone)]
pub struct GradleProject {
    pub root: PathBuf,
    pub subprojects: Vec<Subproject>,
}

/// A Gradle subproject with application-related configuration.
#[derive(Debug, Clone)]
pub struct Subproject {
    pub name: String,
    pub path: PathBuf,
    pub has_application: bool,
    pub main_class: Option<String>,
    pub add_modules: Vec<String>,
}

impl GradleProject {
    /// Parse a Gradle project from the given root directory.
    /// Returns None if not a valid Gradle project.
    pub fn parse(root: &Path) -> Option<Self> {
        // Check for Gradle project markers
        let has_settings_kts = root.join("settings.gradle.kts").exists();
        let has_settings = root.join("settings.gradle").exists();
        let has_build_kts = root.join("build.gradle.kts").exists();
        let has_build = root.join("build.gradle").exists();

        if !has_settings_kts && !has_settings && !has_build_kts && !has_build {
            return None;
        }

        let mut subprojects = Vec::new();

        // Parse settings.gradle.kts or settings.gradle for includes
        let settings_path = if has_settings_kts {
            root.join("settings.gradle.kts")
        } else if has_settings {
            root.join("settings.gradle")
        } else {
            PathBuf::new()
        };

        if settings_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&settings_path) {
                let included = parse_includes(&content);
                for name in included {
                    let subproject_path = root.join(name.replace(':', "/"));
                    if let Some(sub) = parse_subproject(&name, &subproject_path) {
                        subprojects.push(sub);
                    }
                }
            }
        }

        // Fallback: if no subprojects found via include(), scan directories
        // This handles custom plugins like JabRef's javaModules
        if subprojects.is_empty() {
            subprojects = scan_directories_for_subprojects(root);
        }

        // Also check root project itself
        if has_build_kts || has_build {
            let build_path = if has_build_kts {
                root.join("build.gradle.kts")
            } else {
                root.join("build.gradle")
            };
            if let Some(sub) = parse_build_gradle("(root)", &build_path) {
                // Only add root if it has application plugin
                if sub.has_application {
                    let mut sub = sub;
                    sub.path = root.to_path_buf();
                    subprojects.push(sub);
                }
            }
        }

        Some(GradleProject {
            root: root.to_path_buf(),
            subprojects,
        })
    }

    /// Returns subprojects that have the application plugin.
    pub fn application_subprojects(&self) -> Vec<&Subproject> {
        self.subprojects
            .iter()
            .filter(|s| s.has_application)
            .collect()
    }

    /// Check if this is a multi-project build.
    pub fn is_multi_project(&self) -> bool {
        self.subprojects.len() > 1 || self.subprojects.first().is_some_and(|s| s.name != "(root)")
    }
}

/// Scan directories for subprojects with build.gradle(.kts).
/// Used as fallback when include() parsing finds nothing (e.g., custom plugins).
fn scan_directories_for_subprojects(root: &Path) -> Vec<Subproject> {
    let mut subprojects = Vec::new();

    // Common directories to skip
    const SKIP_DIRS: &[&str] = &[
        "build",
        "build-logic",
        ".gradle",
        ".git",
        "gradle",
        "buildSrc",
        "node_modules",
        "target",
        ".idea",
        "versions",
        "test-support",
    ];

    let entries = match std::fs::read_dir(root) {
        Ok(entries) => entries,
        Err(_) => return subprojects,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let dir_name = match path.file_name().and_then(|n| n.to_str()) {
            Some(name) => name,
            None => continue,
        };

        // Skip common non-subproject directories and hidden directories
        if SKIP_DIRS.contains(&dir_name) || dir_name.starts_with('.') {
            continue;
        }

        // Check if directory has build.gradle(.kts)
        if let Some(sub) = parse_subproject(dir_name, &path) {
            subprojects.push(sub);
        }
    }

    subprojects
}

/// Parse include statements from settings.gradle(.kts).
/// Note: This explicitly excludes `includeBuild()` which is for composite builds.
fn parse_includes(content: &str) -> Vec<String> {
    let mut includes = Vec::new();

    // Regex to extract project names from quoted strings
    let project_re = Regex::new(r#"["':]+([a-zA-Z0-9_\-:]+)["']"#).unwrap();

    // Process line by line to properly handle comments
    for line in content.lines() {
        let trimmed = line.trim();

        // Skip comments
        if trimmed.starts_with("//") || trimmed.starts_with('#') || trimmed.starts_with('*') {
            continue;
        }

        // Remove inline comments (// ...) before processing
        let code = if let Some(pos) = trimmed.find("//") {
            &trimmed[..pos]
        } else {
            trimmed
        };

        // Must start with "include" but not "includeBuild" or "includeFlat"
        if code.starts_with("include")
            && !code.starts_with("includeBuild")
            && !code.starts_with("includeFlat")
        {
            for cap in project_re.captures_iter(code) {
                if let Some(m) = cap.get(1) {
                    let name = m.as_str().trim_start_matches(':');
                    if !includes.contains(&name.to_string()) {
                        includes.push(name.to_string());
                    }
                }
            }
        }
    }

    includes
}

/// Parse a subproject directory for its build.gradle(.kts).
fn parse_subproject(name: &str, path: &Path) -> Option<Subproject> {
    let build_kts = path.join("build.gradle.kts");
    let build_groovy = path.join("build.gradle");

    if build_kts.exists() {
        parse_build_gradle(name, &build_kts).map(|mut s| {
            s.path = path.to_path_buf();
            s
        })
    } else if build_groovy.exists() {
        parse_build_gradle(name, &build_groovy).map(|mut s| {
            s.path = path.to_path_buf();
            s
        })
    } else {
        None
    }
}

/// Parse a build.gradle(.kts) file for application configuration.
fn parse_build_gradle(name: &str, path: &Path) -> Option<Subproject> {
    let content = std::fs::read_to_string(path).ok()?;

    // Check for application plugin with specific patterns
    // Avoid false positives like `group = "application"`
    let has_application = content.contains("id(\"application\")")
        || content.contains("id 'application'")
        || content.contains("id \"application\"")
        || content.contains("plugin: 'application'")
        || content.contains("apply plugin: 'application'")
        || content.contains("apply plugin: \"application\"");

    let main_class = extract_main_class(&content);
    let add_modules = extract_add_modules(&content);

    Some(Subproject {
        name: name.to_string(),
        path: PathBuf::new(),
        has_application,
        main_class,
        add_modules,
    })
}

/// Extract mainClass.set("...") from build.gradle.kts.
fn extract_main_class(content: &str) -> Option<String> {
    // Kotlin DSL: mainClass.set("com.example.Main")
    let kts_re = Regex::new(r#"mainClass\.set\s*\(\s*["']([^"']+)["']\s*\)"#).ok()?;
    if let Some(cap) = kts_re.captures(content) {
        return cap.get(1).map(|m| m.as_str().to_string());
    }

    // Groovy DSL: mainClass = 'com.example.Main' or mainClassName = 'com.example.Main'
    let groovy_re = Regex::new(r#"mainClass(?:Name)?\s*=\s*["']([^"']+)["']"#).ok()?;
    if let Some(cap) = groovy_re.captures(content) {
        return cap.get(1).map(|m| m.as_str().to_string());
    }

    None
}

/// Extract addModules.add("...") entries from build.gradle.kts.
fn extract_add_modules(content: &str) -> Vec<String> {
    let mut modules = Vec::new();

    // Kotlin DSL: addModules.add("jdk.incubator.vector")
    let add_re = Regex::new(r#"addModules\.add\s*\(\s*["']([^"']+)["']\s*\)"#).unwrap();
    for cap in add_re.captures_iter(content) {
        if let Some(m) = cap.get(1) {
            modules.push(m.as_str().to_string());
        }
    }

    // Also check for addModules.addAll(listOf(...))
    let addall_re = Regex::new(r#"addModules\.addAll\s*\(\s*listOf\s*\(([^)]+)\)\s*\)"#).unwrap();
    let module_re = Regex::new(r#"["']([^"']+)["']"#).unwrap();
    for cap in addall_re.captures_iter(content) {
        if let Some(m) = cap.get(1) {
            let inner = m.as_str();
            for mcap in module_re.captures_iter(inner) {
                if let Some(mm) = mcap.get(1) {
                    modules.push(mm.as_str().to_string());
                }
            }
        }
    }

    modules
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn parse_includes_jabref_style() {
        // JabRef uses javaModules plugin instead of include()
        // The commented include line should NOT be parsed
        let content = r#"
pluginManagement {
    includeBuild("build-logic")
}

plugins {
    id("org.jabref.gradle.build")
}

rootProject.name = "JabRef"

javaModules {
    directory(".")
    versions("versions")
    // include("jablib", "jabkit", "jabgui", "jabsrv", "jabsrv-cli", "test-support", "versions")
}
"#;
        let includes = parse_includes(content);
        // Should be empty - includeBuild should not be captured, commented include should be ignored
        assert!(includes.is_empty(), "Expected empty, got: {:?}", includes);
    }

    #[test]
    fn parse_includes_kotlin_dsl() {
        let content = r#"
rootProject.name = "myproject"
include("module1")
include("module2", "module3")
include(":nested:submodule")
"#;
        let includes = parse_includes(content);
        assert!(includes.contains(&"module1".to_string()));
        assert!(includes.contains(&"module2".to_string()));
        assert!(includes.contains(&"nested:submodule".to_string()));
    }

    #[test]
    fn parse_includes_groovy_dsl() {
        let content = r#"
rootProject.name = 'myproject'
include 'module1'
include ':module2', ':module3'
"#;
        let includes = parse_includes(content);
        assert!(includes.contains(&"module1".to_string()));
        assert!(includes.contains(&"module2".to_string()));
    }

    #[test]
    fn extract_main_class_kotlin() {
        let content = r#"
application {
    mainClass.set("com.example.Main")
}
"#;
        assert_eq!(
            extract_main_class(content),
            Some("com.example.Main".to_string())
        );
    }

    #[test]
    fn extract_main_class_groovy() {
        let content = r#"
application {
    mainClassName = 'com.example.Main'
}
"#;
        assert_eq!(
            extract_main_class(content),
            Some("com.example.Main".to_string())
        );
    }

    #[test]
    fn extract_add_modules_single() {
        let content = r#"
javaModulePackaging {
    addModules.add("jdk.incubator.vector")
}
"#;
        let modules = extract_add_modules(content);
        assert_eq!(modules, vec!["jdk.incubator.vector"]);
    }

    #[test]
    fn extract_add_modules_multiple() {
        let content = r#"
javaModulePackaging {
    addModules.add("jdk.incubator.vector")
    addModules.add("jdk.incubator.foreign")
}
"#;
        let modules = extract_add_modules(content);
        assert!(modules.contains(&"jdk.incubator.vector".to_string()));
        assert!(modules.contains(&"jdk.incubator.foreign".to_string()));
    }

    #[test]
    fn extract_add_modules_addall() {
        let content = r#"
javaModulePackaging {
    addModules.addAll(listOf("jdk.incubator.vector", "jdk.unsupported"))
}
"#;
        let modules = extract_add_modules(content);
        assert!(modules.contains(&"jdk.incubator.vector".to_string()));
        assert!(modules.contains(&"jdk.unsupported".to_string()));
    }

    #[test]
    fn parse_build_gradle_with_application() {
        let dir = tempdir().unwrap();
        let build_file = dir.path().join("build.gradle.kts");
        std::fs::write(
            &build_file,
            r#"
plugins {
    id("application")
}

application {
    mainClass.set("com.example.Main")
}

javaModulePackaging {
    addModules.add("jdk.incubator.vector")
}
"#,
        )
        .unwrap();

        let sub = parse_build_gradle("test", &build_file).unwrap();
        assert!(sub.has_application);
        assert_eq!(sub.main_class, Some("com.example.Main".to_string()));
        assert_eq!(sub.add_modules, vec!["jdk.incubator.vector"]);
    }

    #[test]
    fn parse_gradle_project_single_module() {
        let dir = tempdir().unwrap();
        std::fs::write(
            dir.path().join("build.gradle.kts"),
            r#"
plugins {
    id("application")
}
application {
    mainClass.set("com.example.Main")
}
"#,
        )
        .unwrap();

        let project = GradleProject::parse(dir.path()).unwrap();
        assert_eq!(project.application_subprojects().len(), 1);
        assert_eq!(project.application_subprojects()[0].name, "(root)");
    }

    #[test]
    fn parse_gradle_project_multi_module() {
        let dir = tempdir().unwrap();

        // settings.gradle.kts
        std::fs::write(
            dir.path().join("settings.gradle.kts"),
            r#"
rootProject.name = "myproject"
include("app")
include("lib")
"#,
        )
        .unwrap();

        // Root build.gradle.kts (no application)
        std::fs::write(
            dir.path().join("build.gradle.kts"),
            r#"
plugins {
    id("java")
}
"#,
        )
        .unwrap();

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

        // lib subproject without application
        let lib_dir = dir.path().join("lib");
        std::fs::create_dir_all(&lib_dir).unwrap();
        std::fs::write(
            lib_dir.join("build.gradle.kts"),
            r#"
plugins {
    id("java-library")
}
"#,
        )
        .unwrap();

        let project = GradleProject::parse(dir.path()).unwrap();
        assert!(project.is_multi_project());

        let apps = project.application_subprojects();
        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].name, "app");
        assert_eq!(apps[0].main_class, Some("com.example.App".to_string()));
    }

    #[test]
    fn parse_gradle_project_with_custom_plugin() {
        // Simulates JabRef's javaModules plugin - no include() but directories exist
        let dir = tempdir().unwrap();

        // settings.gradle.kts with custom plugin (no include statements)
        std::fs::write(
            dir.path().join("settings.gradle.kts"),
            r#"
pluginManagement {
    includeBuild("build-logic")
}

plugins {
    id("org.jabref.gradle.build")
}

rootProject.name = "JabRef"

javaModules {
    directory(".")
    versions("versions")
}
"#,
        )
        .unwrap();

        // Root build.gradle.kts (no application)
        std::fs::write(
            dir.path().join("build.gradle.kts"),
            r#"
plugins {
    id("java")
}
"#,
        )
        .unwrap();

        // jabkit subproject with application
        let jabkit_dir = dir.path().join("jabkit");
        std::fs::create_dir_all(&jabkit_dir).unwrap();
        std::fs::write(
            jabkit_dir.join("build.gradle.kts"),
            r#"
plugins {
    id("application")
}
application {
    mainClass.set("org.jabref.cli.JabKit")
}
"#,
        )
        .unwrap();

        // jablib subproject without application (library)
        let jablib_dir = dir.path().join("jablib");
        std::fs::create_dir_all(&jablib_dir).unwrap();
        std::fs::write(
            jablib_dir.join("build.gradle.kts"),
            r#"
plugins {
    id("java-library")
}
"#,
        )
        .unwrap();

        // build-logic should be skipped
        let build_logic_dir = dir.path().join("build-logic");
        std::fs::create_dir_all(&build_logic_dir).unwrap();
        std::fs::write(
            build_logic_dir.join("build.gradle.kts"),
            r#"
plugins {
    id("java-gradle-plugin")
}
"#,
        )
        .unwrap();

        let project = GradleProject::parse(dir.path()).unwrap();

        // Should detect jabkit via directory scan fallback
        let apps = project.application_subprojects();
        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].name, "jabkit");
        assert_eq!(
            apps[0].main_class,
            Some("org.jabref.cli.JabKit".to_string())
        );
    }

    #[test]
    fn application_subprojects_filters_correctly() {
        let project = GradleProject {
            root: PathBuf::from("/test"),
            subprojects: vec![
                Subproject {
                    name: "app".to_string(),
                    path: PathBuf::from("/test/app"),
                    has_application: true,
                    main_class: Some("Main".to_string()),
                    add_modules: vec![],
                },
                Subproject {
                    name: "lib".to_string(),
                    path: PathBuf::from("/test/lib"),
                    has_application: false,
                    main_class: None,
                    add_modules: vec![],
                },
            ],
        };

        let apps = project.application_subprojects();
        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].name, "app");
    }
}
