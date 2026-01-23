use std::path::{Path, PathBuf};
use std::process::Command;

use crate::config::BuildSystem;
use crate::error::PackError;

fn ensure_command_exists(cmd: &str) -> Result<(), PackError> {
    which::which(cmd).map_err(|_| {
        PackError::BuildFailed(format!(
            "command '{cmd}' not found in PATH. Please install it before running jbundle."
        ))
    })?;
    Ok(())
}

pub fn build_uberjar(project_dir: &Path, system: BuildSystem) -> Result<PathBuf, PackError> {
    match system {
        BuildSystem::DepsEdn => build_deps_edn(project_dir),
        BuildSystem::Leiningen => build_leiningen(project_dir),
        BuildSystem::Maven => build_maven(project_dir),
        BuildSystem::Gradle => build_gradle(project_dir),
    }
}

fn build_deps_edn(project_dir: &Path) -> Result<PathBuf, PackError> {
    let strategy = detect_deps_strategy(project_dir);
    let args = strategy.to_args();
    run_clojure_command(project_dir, &args)
}

#[derive(Debug, PartialEq)]
enum DepsStrategy {
    ToolsBuild {
        function: String,
    },
    ToolsBuildAlias {
        alias: String,
        function: String,
    },
    MainFunction {
        alias: String,
        namespace: String,
        args: Vec<String>,
    },
    Uberjar,
}

impl DepsStrategy {
    fn to_args(&self) -> Vec<String> {
        match self {
            DepsStrategy::ToolsBuild { function } => {
                vec![format!("-T:build"), function.clone()]
            }
            DepsStrategy::ToolsBuildAlias { alias, function } => {
                vec![format!("-T:{alias}"), function.clone()]
            }
            DepsStrategy::MainFunction {
                alias,
                namespace,
                args,
            } => {
                let mut result = vec![format!("-M:{alias}"), "-m".to_string(), namespace.clone()];
                result.extend(args.iter().cloned());
                result
            }
            DepsStrategy::Uberjar => {
                vec!["-X:uberjar".to_string()]
            }
        }
    }
}

#[derive(Debug)]
struct ParsedAlias {
    name: String,
    has_ns_default: bool,
    extra_paths: Vec<String>,
}

#[derive(Debug)]
struct BuildFileInfo {
    namespace: String,
    has_uber_fn: Option<String>,
    has_main_fn: bool,
    has_b_uber_call: bool,
}

fn detect_deps_strategy(project_dir: &Path) -> DepsStrategy {
    // 1. Root build.clj exists → parse for defn uber/uberjar
    let build_clj = project_dir.join("build.clj");
    if build_clj.exists() {
        if let Ok(content) = std::fs::read_to_string(&build_clj) {
            let function = detect_build_function(&content).unwrap_or_else(|| "uber".to_string());
            return DepsStrategy::ToolsBuild { function };
        }
        return DepsStrategy::ToolsBuild {
            function: "uber".to_string(),
        };
    }

    // 2. Parse deps.edn for aliases with tools.build
    let deps_path = project_dir.join("deps.edn");
    if let Ok(deps_content) = std::fs::read_to_string(&deps_path) {
        let aliases = parse_aliases_with_tools_build(&deps_content);

        for alias in &aliases {
            // 2a. Alias with :ns-default
            if alias.has_ns_default {
                return DepsStrategy::ToolsBuildAlias {
                    alias: alias.name.clone(),
                    function: "uber".to_string(),
                };
            }

            // 2b. Alias with :extra-paths → scan for build files
            if !alias.extra_paths.is_empty() {
                let build_files = find_build_files(project_dir, &alias.extra_paths);
                for bf in &build_files {
                    if bf.has_main_fn && bf.has_b_uber_call {
                        return DepsStrategy::MainFunction {
                            alias: alias.name.clone(),
                            namespace: bf.namespace.clone(),
                            args: vec!["--uberjar".to_string()],
                        };
                    }
                    if let Some(ref fn_name) = bf.has_uber_fn {
                        return DepsStrategy::ToolsBuildAlias {
                            alias: alias.name.clone(),
                            function: fn_name.clone(),
                        };
                    }
                }
            }
        }

        // 3. :uberjar alias in deps.edn
        if deps_content.contains(":uberjar") {
            return DepsStrategy::Uberjar;
        }
    }

    // 4. Fallback
    DepsStrategy::ToolsBuild {
        function: "uber".to_string(),
    }
}

fn run_clojure_command(project_dir: &Path, args: &[String]) -> Result<PathBuf, PackError> {
    ensure_command_exists("clojure")?;
    let cmd_str = format!("clojure {}", args.join(" "));
    tracing::info!("running: {cmd_str}");

    let output = Command::new("clojure")
        .args(args)
        .current_dir(project_dir)
        .output()
        .map_err(|e| PackError::BuildFailed(format!("failed to run clojure: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let formatted = crate::diagnostic::format_build_error(
            &stderr,
            &stdout,
            BuildSystem::DepsEdn,
            project_dir,
        );
        return Err(PackError::BuildFailed(format!(
            "{cmd_str} failed:\n{formatted}"
        )));
    }

    find_uberjar(project_dir)
}

/// Detects `(defn uber` or `(defn uberjar` in a Clojure build file.
fn detect_build_function(content: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(after_defn) = trimmed.strip_prefix("(defn ") {
            let fn_name: String = after_defn
                .chars()
                .take_while(|c| !c.is_whitespace() && *c != '[' && *c != '(')
                .collect();
            if fn_name == "uber" || fn_name == "uberjar" {
                return Some(fn_name);
            }
        }
    }
    None
}

/// Parses deps.edn content to find aliases that depend on tools.build.
fn parse_aliases_with_tools_build(deps_content: &str) -> Vec<ParsedAlias> {
    let mut result = Vec::new();

    let aliases_pos = match deps_content.find(":aliases") {
        Some(pos) => pos,
        None => return result,
    };

    // Find the opening brace of :aliases map
    let after_aliases = &deps_content[aliases_pos + 8..];
    let map_start = match after_aliases.find('{') {
        Some(pos) => aliases_pos + 8 + pos,
        None => return result,
    };

    let aliases_block = match extract_balanced_block(deps_content, map_start) {
        Some((start, end)) => &deps_content[start..=end],
        None => return result,
    };

    // Find each alias keyword (e.g., :build, :dev)
    let mut search_pos = 0;
    while search_pos < aliases_block.len() {
        // Find next keyword that starts an alias definition
        let remaining = &aliases_block[search_pos..];
        let colon_pos = match remaining.find(':') {
            Some(pos) => pos,
            None => break,
        };

        let abs_colon = search_pos + colon_pos;
        let after_colon = &aliases_block[abs_colon + 1..];

        // Extract alias name
        let alias_name: String = after_colon
            .chars()
            .take_while(|c| !c.is_whitespace() && *c != '{')
            .collect();

        if alias_name.is_empty() || alias_name.contains('/') {
            search_pos = abs_colon + 1;
            continue;
        }

        // Find the alias's map block
        let after_name = abs_colon + 1 + alias_name.len();
        let alias_map_start = match aliases_block[after_name..].find('{') {
            Some(pos) => after_name + pos,
            None => {
                search_pos = after_name;
                continue;
            }
        };

        let alias_block = match extract_balanced_block(aliases_block, alias_map_start) {
            Some((start, end)) => {
                search_pos = end + 1;
                &aliases_block[start..=end]
            }
            None => {
                search_pos = alias_map_start + 1;
                continue;
            }
        };

        // Check if this alias has tools.build as a dependency
        let has_tools_build = alias_block.contains("tools.build")
            || alias_block.contains("clojure/tools.build")
            || alias_block.contains("io.github.clojure/tools.build");

        if !has_tools_build {
            continue;
        }

        let has_ns_default = alias_block.contains(":ns-default");
        let extra_paths = extract_extra_paths(alias_block);

        result.push(ParsedAlias {
            name: alias_name,
            has_ns_default,
            extra_paths,
        });
    }

    result
}

/// Extracts the list of strings from `:extra-paths ["path1" "path2"]`
fn extract_extra_paths(alias_block: &str) -> Vec<String> {
    let mut paths = Vec::new();
    let ep_pos = match alias_block.find(":extra-paths") {
        Some(pos) => pos,
        None => return paths,
    };

    let after_ep = &alias_block[ep_pos + 12..];
    let bracket_pos = match after_ep.find('[') {
        Some(pos) => pos,
        None => return paths,
    };

    let after_bracket = &after_ep[bracket_pos + 1..];
    let close_bracket = match after_bracket.find(']') {
        Some(pos) => pos,
        None => return paths,
    };

    let inside = &after_bracket[..close_bracket];
    // Extract quoted strings
    let mut in_quote = false;
    let mut current = String::new();
    for ch in inside.chars() {
        match ch {
            '"' if !in_quote => {
                in_quote = true;
                current.clear();
            }
            '"' if in_quote => {
                in_quote = false;
                if !current.is_empty() {
                    paths.push(current.clone());
                }
            }
            _ if in_quote => {
                current.push(ch);
            }
            _ => {}
        }
    }

    paths
}

/// Finds a balanced `{...}` block starting at `start_pos`. Returns (start, end) inclusive.
fn extract_balanced_block(content: &str, start_pos: usize) -> Option<(usize, usize)> {
    let bytes = content.as_bytes();
    if start_pos >= bytes.len() || bytes[start_pos] != b'{' {
        return None;
    }

    let mut depth = 0;
    let mut in_string = false;
    let mut i = start_pos;

    while i < bytes.len() {
        match bytes[i] {
            b'"' if !in_string => in_string = true,
            b'"' if in_string => {
                // Check for escaped quote
                if i > 0 && bytes[i - 1] != b'\\' {
                    in_string = false;
                }
            }
            b'{' if !in_string => depth += 1,
            b'}' if !in_string => {
                depth -= 1;
                if depth == 0 {
                    return Some((start_pos, i));
                }
            }
            _ => {}
        }
        i += 1;
    }

    None
}

/// Analyzes a Clojure file for build-related functions.
fn analyze_build_file(path: &Path, base_path: &Path) -> Option<BuildFileInfo> {
    let content = std::fs::read_to_string(path).ok()?;
    let namespace =
        detect_namespace(&content).unwrap_or_else(|| path_to_namespace(path, base_path));

    let has_uber_fn = detect_build_function(&content);
    let has_main_fn = content.contains("(defn -main");
    let has_b_uber_call = content.contains("b/uber") || content.contains("tools.build.api/uber");

    Some(BuildFileInfo {
        namespace,
        has_uber_fn,
        has_main_fn,
        has_b_uber_call,
    })
}

/// Detects `(ns ...)` declaration and extracts the namespace name.
fn detect_namespace(content: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(after_ns) = trimmed.strip_prefix("(ns ") {
            let ns_name: String = after_ns
                .chars()
                .take_while(|c| !c.is_whitespace() && *c != ')' && *c != '(')
                .collect();
            if !ns_name.is_empty() {
                return Some(ns_name);
            }
        }
    }
    None
}

/// Converts a file path relative to a base into a Clojure namespace.
/// e.g., `dev/com/foo/build.clj` with base `dev` → `com.foo.build`
fn path_to_namespace(file_path: &Path, base_path: &Path) -> String {
    let relative = file_path.strip_prefix(base_path).unwrap_or(file_path);

    let stem = relative.with_extension("");
    stem.to_string_lossy()
        .replace([std::path::MAIN_SEPARATOR, '/'], ".")
        .replace('_', "-")
}

/// Scans extra-paths directories for .clj files that look like build files.
fn find_build_files(project_dir: &Path, extra_paths: &[String]) -> Vec<BuildFileInfo> {
    let mut results = Vec::new();

    for ep in extra_paths {
        let ep_dir = project_dir.join(ep);
        if !ep_dir.is_dir() {
            // Maybe it's a file directly
            if ep_dir.extension().is_some_and(|e| e == "clj") {
                if let Some(info) = analyze_build_file(&ep_dir, project_dir) {
                    results.push(info);
                }
            }
            continue;
        }

        if let Ok(entries) = walk_clj_files(&ep_dir) {
            for file_path in entries {
                if let Some(info) = analyze_build_file(&file_path, &ep_dir) {
                    results.push(info);
                }
            }
        }
    }

    results
}

/// Recursively walks a directory for .clj files.
fn walk_clj_files(dir: &Path) -> Result<Vec<PathBuf>, std::io::Error> {
    let mut files = Vec::new();
    walk_clj_files_recursive(dir, &mut files)?;
    Ok(files)
}

fn walk_clj_files_recursive(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), std::io::Error> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            walk_clj_files_recursive(&path, files)?;
        } else if path.extension().is_some_and(|e| e == "clj") {
            files.push(path);
        }
    }
    Ok(())
}

fn build_leiningen(project_dir: &Path) -> Result<PathBuf, PackError> {
    ensure_command_exists("lein")?;
    tracing::info!("running: lein uberjar");

    let output = Command::new("lein")
        .arg("uberjar")
        .current_dir(project_dir)
        .output()
        .map_err(|e| PackError::BuildFailed(format!("failed to run lein: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let formatted = crate::diagnostic::format_build_error(
            &stderr,
            &stdout,
            BuildSystem::Leiningen,
            project_dir,
        );
        return Err(PackError::BuildFailed(format!(
            "lein uberjar failed:\n{formatted}"
        )));
    }

    find_uberjar(project_dir)
}

fn build_maven(project_dir: &Path) -> Result<PathBuf, PackError> {
    ensure_command_exists("mvn")?;
    tracing::info!("running: mvn package -DskipTests");

    let output = Command::new("mvn")
        .args(["package", "-DskipTests"])
        .current_dir(project_dir)
        .output()
        .map_err(|e| PackError::BuildFailed(format!("failed to run mvn: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let formatted = crate::diagnostic::format_build_error(
            &stderr,
            &stdout,
            BuildSystem::Maven,
            project_dir,
        );
        return Err(PackError::BuildFailed(format!(
            "mvn package failed:\n{formatted}"
        )));
    }

    find_uberjar(project_dir)
}

fn build_gradle(project_dir: &Path) -> Result<PathBuf, PackError> {
    let (cmd, cmd_name) = if project_dir.join("gradlew").exists() {
        ("./gradlew".to_string(), "gradlew")
    } else {
        ensure_command_exists("gradle")?;
        ("gradle".to_string(), "gradle")
    };

    tracing::info!("running: {cmd_name} build -x test");

    let output = Command::new(&cmd)
        .args(["build", "-x", "test"])
        .current_dir(project_dir)
        .output()
        .map_err(|e| PackError::BuildFailed(format!("failed to run {cmd_name}: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let formatted = crate::diagnostic::format_build_error(
            &stderr,
            &stdout,
            BuildSystem::Gradle,
            project_dir,
        );
        return Err(PackError::BuildFailed(format!(
            "{cmd_name} build failed:\n{formatted}"
        )));
    }

    find_jar_in_dirs(project_dir, &["build/libs", "target"])
}

fn find_jar_in_dirs(project_dir: &Path, dirs: &[&str]) -> Result<PathBuf, PackError> {
    for dir in dirs {
        let target_dir = project_dir.join(dir);
        if target_dir.exists() {
            if let Some(jar) = find_best_jar(&target_dir) {
                return Ok(jar);
            }
        }
    }
    Err(PackError::UberjarNotFound(project_dir.join(dirs[0])))
}

fn find_uberjar(project_dir: &Path) -> Result<PathBuf, PackError> {
    let target_dir = project_dir.join("target");
    find_best_jar(&target_dir).ok_or_else(|| PackError::UberjarNotFound(target_dir))
}

/// Finds the best JAR in a directory, preferring uber/standalone jars, then any jar
/// (excluding sources/javadoc). Returns the most recently modified match.
fn find_best_jar(dir: &Path) -> Option<PathBuf> {
    // Prefer uber/standalone jars
    if let Ok(entries) = std::fs::read_dir(dir) {
        let mut candidates: Vec<PathBuf> = entries
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.extension().is_some_and(|ext| ext == "jar")
                    && p.file_name().and_then(|n| n.to_str()).is_some_and(|n| {
                        n.contains("standalone")
                            || n.contains("uber")
                            || n.contains("jar-with-dependencies")
                            || n.contains("-all")
                            || n.contains("-fat")
                    })
            })
            .collect();

        candidates.sort_by_key(|p| std::fs::metadata(p).and_then(|m| m.modified()).ok());
        candidates.reverse();

        if let Some(jar) = candidates.into_iter().next() {
            return Some(jar);
        }
    }

    // Fall back to any jar (excluding sources/javadoc/plain/original)
    if let Ok(entries) = std::fs::read_dir(dir) {
        let mut jars: Vec<PathBuf> = entries
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.extension().is_some_and(|ext| ext == "jar")
                    && p.file_name().and_then(|n| n.to_str()).is_some_and(|n| {
                        !n.contains("sources")
                            && !n.contains("javadoc")
                            && !n.contains("-plain")
                            && !n.contains(".original")
                    })
            })
            .collect();

        jars.sort_by_key(|p| std::fs::metadata(p).and_then(|m| m.modified()).ok());
        jars.reverse();

        if let Some(jar) = jars.into_iter().next() {
            tracing::warn!(
                "no uber/standalone JAR found, using '{}'. If the app has external dependencies, \
                 configure your build to produce a fat JAR (maven-shade-plugin, shadow jar, etc.)",
                jar.file_name().unwrap_or_default().to_string_lossy()
            );
            return Some(jar);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn find_uberjar_prefers_standalone_jar() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("target");
        std::fs::create_dir_all(&target).unwrap();
        std::fs::write(target.join("app.jar"), b"regular").unwrap();
        std::fs::write(target.join("app-standalone.jar"), b"standalone").unwrap();

        let result = find_uberjar(dir.path()).unwrap();
        assert!(result
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .contains("standalone"));
    }

    #[test]
    fn find_uberjar_prefers_uber_jar() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("target");
        std::fs::create_dir_all(&target).unwrap();
        std::fs::write(target.join("app.jar"), b"regular").unwrap();
        std::fs::write(target.join("app-uber.jar"), b"uber").unwrap();

        let result = find_uberjar(dir.path()).unwrap();
        assert!(result
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .contains("uber"));
    }

    #[test]
    fn find_uberjar_falls_back_to_any_jar() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("target");
        std::fs::create_dir_all(&target).unwrap();
        std::fs::write(target.join("myapp.jar"), b"content").unwrap();

        let result = find_uberjar(dir.path()).unwrap();
        assert_eq!(result.file_name().unwrap(), "myapp.jar");
    }

    #[test]
    fn find_uberjar_excludes_sources_and_javadoc() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("target");
        std::fs::create_dir_all(&target).unwrap();
        std::fs::write(target.join("app-sources.jar"), b"src").unwrap();
        std::fs::write(target.join("app-javadoc.jar"), b"doc").unwrap();
        std::fs::write(target.join("app.jar"), b"app").unwrap();

        let result = find_uberjar(dir.path()).unwrap();
        assert_eq!(result.file_name().unwrap(), "app.jar");
    }

    #[test]
    fn find_uberjar_error_when_no_jars() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("target");
        std::fs::create_dir_all(&target).unwrap();
        std::fs::write(target.join("readme.txt"), b"text").unwrap();

        let result = find_uberjar(dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn find_uberjar_error_when_no_target_dir() {
        let dir = tempdir().unwrap();
        let result = find_uberjar(dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn detect_strategy_root_build_clj_with_uber() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("deps.edn"), "{}").unwrap();
        std::fs::write(
            dir.path().join("build.clj"),
            "(ns build\n  (:require [clojure.tools.build.api :as b]))\n\n(defn uber [_]\n  (b/uber {}))\n",
        ).unwrap();

        let strategy = detect_deps_strategy(dir.path());
        assert_eq!(
            strategy,
            DepsStrategy::ToolsBuild {
                function: "uber".to_string()
            }
        );
    }

    #[test]
    fn detect_strategy_root_build_clj_with_uberjar() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("deps.edn"), "{}").unwrap();
        std::fs::write(
            dir.path().join("build.clj"),
            "(ns build)\n\n(defn uberjar [opts]\n  (println \"building\"))\n",
        )
        .unwrap();

        let strategy = detect_deps_strategy(dir.path());
        assert_eq!(
            strategy,
            DepsStrategy::ToolsBuild {
                function: "uberjar".to_string()
            }
        );
    }

    #[test]
    fn detect_strategy_root_build_clj_no_uber_fn_defaults_uber() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("deps.edn"), "{}").unwrap();
        std::fs::write(
            dir.path().join("build.clj"),
            "(ns build)\n\n(defn clean [_]\n  (println \"clean\"))\n",
        )
        .unwrap();

        let strategy = detect_deps_strategy(dir.path());
        assert_eq!(
            strategy,
            DepsStrategy::ToolsBuild {
                function: "uber".to_string()
            }
        );
    }

    #[test]
    fn detect_strategy_alias_with_ns_default() {
        let dir = tempdir().unwrap();
        let deps = r#"{:paths ["src"]
 :aliases
 {:build
  {:deps {io.github.clojure/tools.build {:mvn/version "0.10.7"}}
   :ns-default build}}}"#;
        std::fs::write(dir.path().join("deps.edn"), deps).unwrap();

        let strategy = detect_deps_strategy(dir.path());
        assert_eq!(
            strategy,
            DepsStrategy::ToolsBuildAlias {
                alias: "build".to_string(),
                function: "uber".to_string(),
            }
        );
    }

    #[test]
    fn detect_strategy_alias_extra_path_with_main_and_b_uber() {
        let dir = tempdir().unwrap();
        let deps = r#"{:paths ["src"]
 :aliases
 {:dev
  {:deps {io.github.clojure/tools.build {:mvn/version "0.10.7"}}
   :extra-paths ["dev"]}}}"#;
        std::fs::write(dir.path().join("deps.edn"), deps).unwrap();

        let dev_dir = dir.path().join("dev");
        std::fs::create_dir_all(&dev_dir).unwrap();
        std::fs::write(
            dev_dir.join("build.clj"),
            "(ns build\n  (:require [clojure.tools.build.api :as b]))\n\n(defn -main [& args]\n  (b/uber {:uber-file \"target/app.jar\"}))\n",
        ).unwrap();

        let strategy = detect_deps_strategy(dir.path());
        assert_eq!(
            strategy,
            DepsStrategy::MainFunction {
                alias: "dev".to_string(),
                namespace: "build".to_string(),
                args: vec!["--uberjar".to_string()],
            }
        );
    }

    #[test]
    fn detect_strategy_alias_extra_path_with_uber_fn() {
        let dir = tempdir().unwrap();
        let deps = r#"{:paths ["src"]
 :aliases
 {:dev
  {:deps {io.github.clojure/tools.build {:mvn/version "0.10.7"}}
   :extra-paths ["dev"]}}}"#;
        std::fs::write(dir.path().join("deps.edn"), deps).unwrap();

        let dev_dir = dir.path().join("dev");
        std::fs::create_dir_all(&dev_dir).unwrap();
        std::fs::write(
            dev_dir.join("build.clj"),
            "(ns build\n  (:require [clojure.tools.build.api :as b]))\n\n(defn uber [_]\n  (b/uber {}))\n",
        ).unwrap();

        let strategy = detect_deps_strategy(dir.path());
        assert_eq!(
            strategy,
            DepsStrategy::ToolsBuildAlias {
                alias: "dev".to_string(),
                function: "uber".to_string(),
            }
        );
    }

    #[test]
    fn detect_strategy_uberjar_alias() {
        let dir = tempdir().unwrap();
        std::fs::write(
            dir.path().join("deps.edn"),
            "{:aliases {:uberjar {:some :config}}}",
        )
        .unwrap();

        let strategy = detect_deps_strategy(dir.path());
        assert_eq!(strategy, DepsStrategy::Uberjar);
    }

    #[test]
    fn detect_strategy_fallback() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("deps.edn"), "{:paths [\"src\"]}").unwrap();

        let strategy = detect_deps_strategy(dir.path());
        assert_eq!(
            strategy,
            DepsStrategy::ToolsBuild {
                function: "uber".to_string()
            }
        );
    }

    #[test]
    fn detect_build_function_finds_uber() {
        let content = "(ns build)\n\n(defn clean [_] nil)\n\n(defn uber [opts]\n  (b/uber opts))\n";
        assert_eq!(detect_build_function(content), Some("uber".to_string()));
    }

    #[test]
    fn detect_build_function_finds_uberjar() {
        let content = "(ns build)\n\n(defn uberjar [_] nil)\n";
        assert_eq!(detect_build_function(content), Some("uberjar".to_string()));
    }

    #[test]
    fn detect_build_function_none_when_missing() {
        let content = "(ns build)\n\n(defn clean [_] nil)\n(defn compile [_] nil)\n";
        assert_eq!(detect_build_function(content), None);
    }

    #[test]
    fn path_to_namespace_simple() {
        let base = Path::new("/project/dev");
        let file = Path::new("/project/dev/build.clj");
        assert_eq!(path_to_namespace(file, base), "build");
    }

    #[test]
    fn path_to_namespace_nested() {
        let base = Path::new("/project/dev");
        let file = Path::new("/project/dev/com/foo/build.clj");
        assert_eq!(path_to_namespace(file, base), "com.foo.build");
    }

    #[test]
    fn path_to_namespace_with_underscores() {
        let base = Path::new("/project/src");
        let file = Path::new("/project/src/my_app/core.clj");
        assert_eq!(path_to_namespace(file, base), "my-app.core");
    }

    #[test]
    fn extract_balanced_block_simple() {
        let content = "{:a 1}";
        assert_eq!(extract_balanced_block(content, 0), Some((0, 5)));
    }

    #[test]
    fn extract_balanced_block_nested() {
        let content = "{:a {:b 2} :c 3}";
        assert_eq!(extract_balanced_block(content, 0), Some((0, 15)));
    }

    #[test]
    fn extract_balanced_block_with_strings() {
        let content = r#"{:a "hello {world}" :b 1}"#;
        assert_eq!(extract_balanced_block(content, 0), Some((0, 24)));
    }

    #[test]
    fn extract_balanced_block_not_at_brace() {
        let content = "abc{def}";
        assert_eq!(extract_balanced_block(content, 0), None);
        assert_eq!(extract_balanced_block(content, 3), Some((3, 7)));
    }

    #[test]
    fn parse_aliases_with_tools_build_finds_build_alias() {
        let deps = r#"{:aliases {:build {:deps {io.github.clojure/tools.build {:mvn/version "0.10.7"}} :ns-default build}}}"#;
        let aliases = parse_aliases_with_tools_build(deps);
        assert_eq!(aliases.len(), 1);
        assert_eq!(aliases[0].name, "build");
        assert!(aliases[0].has_ns_default);
    }

    #[test]
    fn parse_aliases_with_tools_build_finds_extra_paths() {
        let deps = r#"{:aliases {:dev {:deps {io.github.clojure/tools.build {:mvn/version "0.10.7"}} :extra-paths ["dev" "test"]}}}"#;
        let aliases = parse_aliases_with_tools_build(deps);
        assert_eq!(aliases.len(), 1);
        assert_eq!(aliases[0].name, "dev");
        assert!(!aliases[0].has_ns_default);
        assert_eq!(aliases[0].extra_paths, vec!["dev", "test"]);
    }

    #[test]
    fn parse_aliases_skips_non_tools_build() {
        let deps = r#"{:aliases {:test {:deps {lambdaisland/kaocha {:mvn/version "1.0"}}}}}"#;
        let aliases = parse_aliases_with_tools_build(deps);
        assert!(aliases.is_empty());
    }

    #[test]
    fn strategy_to_args_tools_build() {
        let s = DepsStrategy::ToolsBuild {
            function: "uber".to_string(),
        };
        assert_eq!(s.to_args(), vec!["-T:build", "uber"]);
    }

    #[test]
    fn strategy_to_args_tools_build_alias() {
        let s = DepsStrategy::ToolsBuildAlias {
            alias: "dev".to_string(),
            function: "uberjar".to_string(),
        };
        assert_eq!(s.to_args(), vec!["-T:dev", "uberjar"]);
    }

    #[test]
    fn strategy_to_args_main_function() {
        let s = DepsStrategy::MainFunction {
            alias: "dev".to_string(),
            namespace: "com.foo.build".to_string(),
            args: vec!["--uberjar".to_string()],
        };
        assert_eq!(
            s.to_args(),
            vec!["-M:dev", "-m", "com.foo.build", "--uberjar"]
        );
    }

    #[test]
    fn strategy_to_args_uberjar() {
        let s = DepsStrategy::Uberjar;
        assert_eq!(s.to_args(), vec!["-X:uberjar"]);
    }

    #[test]
    fn detect_namespace_from_ns_form() {
        let content = "(ns com.example.build\n  (:require [clojure.tools.build.api :as b]))\n";
        assert_eq!(
            detect_namespace(content),
            Some("com.example.build".to_string())
        );
    }

    #[test]
    fn detect_namespace_none_when_missing() {
        let content = "(defn foo [] nil)\n";
        assert_eq!(detect_namespace(content), None);
    }

    #[test]
    fn ensure_command_exists_finds_sh() {
        assert!(ensure_command_exists("sh").is_ok());
    }

    #[test]
    fn ensure_command_exists_fails_for_missing() {
        assert!(ensure_command_exists("nonexistent_binary_xyz_123").is_err());
    }

    #[test]
    fn find_best_jar_prefers_jar_with_dependencies() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("target");
        std::fs::create_dir_all(&target).unwrap();
        std::fs::write(target.join("app-1.0.jar"), b"regular").unwrap();
        std::fs::write(target.join("app-1.0-jar-with-dependencies.jar"), b"fat").unwrap();

        let result = find_best_jar(&target).unwrap();
        assert!(result.to_str().unwrap().contains("jar-with-dependencies"));
    }

    #[test]
    fn find_best_jar_prefers_all_jar() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("target");
        std::fs::create_dir_all(&target).unwrap();
        std::fs::write(target.join("app-1.0.jar"), b"regular").unwrap();
        std::fs::write(target.join("app-1.0-all.jar"), b"all").unwrap();

        let result = find_best_jar(&target).unwrap();
        assert!(result.to_str().unwrap().contains("-all"));
    }

    #[test]
    fn find_best_jar_excludes_plain_and_original() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("target");
        std::fs::create_dir_all(&target).unwrap();
        std::fs::write(target.join("app-plain.jar"), b"plain").unwrap();
        std::fs::write(target.join("app.jar.original"), b"orig").unwrap();
        std::fs::write(target.join("app.jar"), b"real").unwrap();

        let result = find_best_jar(&target).unwrap();
        assert_eq!(result.file_name().unwrap(), "app.jar");
    }

    #[test]
    fn find_jar_in_dirs_checks_multiple_dirs() {
        let dir = tempdir().unwrap();
        let libs = dir.path().join("build/libs");
        std::fs::create_dir_all(&libs).unwrap();
        std::fs::write(libs.join("app.jar"), b"gradle-jar").unwrap();

        let result = find_jar_in_dirs(dir.path(), &["target", "build/libs"]).unwrap();
        assert_eq!(result.file_name().unwrap(), "app.jar");
    }

    #[test]
    fn find_jar_in_dirs_prefers_first_dir() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("target");
        let libs = dir.path().join("build/libs");
        std::fs::create_dir_all(&target).unwrap();
        std::fs::create_dir_all(&libs).unwrap();
        std::fs::write(target.join("from-maven.jar"), b"maven").unwrap();
        std::fs::write(libs.join("from-gradle.jar"), b"gradle").unwrap();

        let result = find_jar_in_dirs(dir.path(), &["target", "build/libs"]).unwrap();
        assert_eq!(result.file_name().unwrap(), "from-maven.jar");
    }
}
