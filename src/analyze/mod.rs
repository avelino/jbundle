mod classify;

use std::collections::HashMap;
use std::path::Path;

use indicatif::HumanBytes;
use zip::ZipArchive;

use crate::error::PackError;
use classify::{classify_entry, detect_clojure_ns, extract_package, EntryCategory};

#[derive(Debug)]
pub struct AnalysisReport {
    pub jar_path: String,
    pub disk_size: u64,
    pub total_uncompressed: u64,
    pub entry_count: usize,
    pub categories: Vec<CategoryStats>,
    pub top_packages: Vec<(String, u64, usize)>,
    pub clojure_namespaces: Vec<(String, u64, usize)>,
    pub shrink_estimate: ShrinkEstimate,
    pub issues: Vec<AnalysisIssue>,
}

#[derive(Debug)]
pub struct CategoryStats {
    pub name: String,
    pub size: u64,
    pub file_count: usize,
}

#[derive(Debug)]
pub struct ShrinkEstimate {
    pub removable_size: u64,
    pub removable_files: usize,
}

#[derive(Debug)]
pub struct AnalysisIssue {
    pub message: String,
}

const LARGE_RESOURCE_THRESHOLD: u64 = 1_048_576; // 1 MB
const TOP_N: usize = 20;

pub fn analyze_jar(jar_path: &Path) -> Result<AnalysisReport, PackError> {
    let file = std::fs::File::open(jar_path)
        .map_err(|e| PackError::AnalyzeFailed(format!("cannot open JAR: {e}")))?;
    let disk_size = file
        .metadata()
        .map_err(|e| PackError::AnalyzeFailed(format!("cannot stat JAR: {e}")))?
        .len();
    let mut archive = ZipArchive::new(file)
        .map_err(|e| PackError::AnalyzeFailed(format!("cannot read JAR: {e}")))?;

    let mut total_uncompressed: u64 = 0;
    let mut cat_counters: HashMap<&str, (u64, usize)> = HashMap::new();
    let mut packages: HashMap<String, (u64, usize)> = HashMap::new();
    let mut clj_ns_map: HashMap<String, (u64, usize)> = HashMap::new();
    let mut class_occurrences: HashMap<String, usize> = HashMap::new();
    let mut large_resources: Vec<(String, u64)> = Vec::new();
    let mut shrink_size: u64 = 0;
    let mut shrink_count: usize = 0;

    let entry_count = archive.len();

    for i in 0..entry_count {
        let entry = archive
            .by_index(i)
            .map_err(|e| PackError::AnalyzeFailed(format!("zip entry error: {e}")))?;

        let name = entry.name().to_string();
        if entry.is_dir() {
            continue;
        }

        let size = entry.size();
        total_uncompressed += size;
        let category = classify_entry(&name);

        let cat_key = match category {
            EntryCategory::Class => "Classes",
            EntryCategory::Resource => "Resources",
            EntryCategory::NativeLib => "Native libs",
            EntryCategory::Metadata => "Metadata",
            EntryCategory::ClojureSource => "Clojure sources",
            EntryCategory::JavaSource => "Java sources",
        };
        let counter = cat_counters.entry(cat_key).or_insert((0, 0));
        counter.0 += size;
        counter.1 += 1;

        if category == EntryCategory::Class {
            *class_occurrences.entry(name.clone()).or_insert(0) += 1;
            if let Some(ns) = detect_clojure_ns(&name) {
                let e = clj_ns_map.entry(ns).or_insert((0, 0));
                e.0 += size;
                e.1 += 1;
            }
        }

        if category == EntryCategory::Resource && size >= LARGE_RESOURCE_THRESHOLD {
            large_resources.push((name.clone(), size));
        }

        let pkg = extract_package(&name);
        let pkg_entry = packages.entry(pkg).or_insert((0, 0));
        pkg_entry.0 += size;
        pkg_entry.1 += 1;

        if crate::shrink::should_skip(&name) {
            shrink_size += size;
            shrink_count += 1;
        }
    }

    let mut categories: Vec<CategoryStats> = cat_counters
        .into_iter()
        .map(|(name, (size, count))| CategoryStats {
            name: name.to_string(),
            size,
            file_count: count,
        })
        .collect();
    categories.sort_by(|a, b| b.size.cmp(&a.size));

    let mut top_packages: Vec<(String, u64, usize)> =
        packages.into_iter().map(|(k, (s, c))| (k, s, c)).collect();
    top_packages.sort_by(|a, b| b.1.cmp(&a.1));
    top_packages.truncate(TOP_N);

    let mut clojure_namespaces: Vec<(String, u64, usize)> = clj_ns_map
        .into_iter()
        .map(|(k, (s, c))| (k, s, c))
        .collect();
    clojure_namespaces.sort_by(|a, b| b.1.cmp(&a.1));
    clojure_namespaces.truncate(TOP_N);

    let mut issues = Vec::new();
    for (name, count) in &class_occurrences {
        if *count > 1 {
            issues.push(AnalysisIssue {
                message: format!("Duplicate class: {} ({} occurrences)", name, count),
            });
        }
    }
    issues.sort_by(|a, b| a.message.cmp(&b.message));
    for (name, size) in &large_resources {
        issues.push(AnalysisIssue {
            message: format!("Large resource: {} ({})", name, HumanBytes(*size)),
        });
    }

    Ok(AnalysisReport {
        jar_path: jar_path.display().to_string(),
        disk_size,
        total_uncompressed,
        entry_count,
        categories,
        top_packages,
        clojure_namespaces,
        shrink_estimate: ShrinkEstimate {
            removable_size: shrink_size,
            removable_files: shrink_count,
        },
        issues,
    })
}

pub fn render_report(report: &AnalysisReport) {
    eprintln!();
    eprintln!(
        "JAR: {} ({})",
        report.jar_path,
        HumanBytes(report.disk_size)
    );
    eprintln!("Entries: {}", format_number(report.entry_count));
    eprintln!();

    if !report.categories.is_empty() {
        eprintln!(
            "{:<20} {:>10} {:>6} {:>8}",
            "Category", "Size", "%", "Files"
        );
        eprintln!("{}", "\u{2500}".repeat(48));
        for cat in &report.categories {
            let pct = if report.total_uncompressed > 0 {
                (cat.size as f64 / report.total_uncompressed as f64) * 100.0
            } else {
                0.0
            };
            eprintln!(
                "{:<20} {:>10} {:>5.0}% {:>8}",
                cat.name,
                HumanBytes(cat.size),
                pct,
                format_number(cat.file_count),
            );
        }
        eprintln!();
    }

    if !report.top_packages.is_empty() {
        eprintln!("Top packages by size:");
        for (pkg, size, count) in &report.top_packages {
            eprintln!(
                "  {:<35} {:>10}  {} files",
                pkg,
                HumanBytes(*size),
                format_number(*count),
            );
        }
        eprintln!();
    }

    if !report.clojure_namespaces.is_empty() {
        eprintln!("Clojure namespaces:");
        for (ns, size, count) in &report.clojure_namespaces {
            eprintln!(
                "  {:<35} {:>10}  {} files",
                ns,
                HumanBytes(*size),
                format_number(*count),
            );
        }
        eprintln!();
    }

    let est = &report.shrink_estimate;
    if est.removable_files > 0 {
        let pct = if report.total_uncompressed > 0 {
            (est.removable_size as f64 / report.total_uncompressed as f64) * 100.0
        } else {
            0.0
        };
        eprintln!(
            "Estimated --shrink savings: {} ({:.0}%) \u{2014} {} removable files",
            HumanBytes(est.removable_size),
            pct,
            format_number(est.removable_files),
        );
        eprintln!();
    }

    if !report.issues.is_empty() {
        eprintln!("Potential issues:");
        for issue in &report.issues {
            eprintln!("  {}", issue.message);
        }
        eprintln!();
    }
}

pub fn run_analyze(input: &Path) -> Result<(), PackError> {
    let jar_path = if input.extension().is_some_and(|e| e == "jar") {
        input.to_path_buf()
    } else if input.is_dir() {
        let detected = crate::detect::detect_build_system_enhanced(input)?;
        match detected {
            crate::detect::DetectedBuild::Simple(system) => {
                eprintln!("Detected build system: {:?}", system);
                eprintln!("Building uberjar...");
                crate::build::build_uberjar(input, system)?
            }
            crate::detect::DetectedBuild::GradleMultiProject {
                app_subprojects, ..
            } => {
                if app_subprojects.is_empty() {
                    return Err(PackError::AnalyzeFailed(
                        "no application subprojects found".to_string(),
                    ));
                }
                let sub = &app_subprojects[0];
                eprintln!("Detected Gradle multi-project, using: {}", sub.name);
                eprintln!("Building uberjar...");
                crate::build::build_gradle_subproject(input, &sub.name)?
            }
        }
    } else {
        return Err(PackError::AnalyzeFailed(format!(
            "input is not a JAR file or project directory: {}",
            input.display()
        )));
    };

    let report = analyze_jar(&jar_path)?;
    render_report(&report);
    Ok(())
}

fn format_number(n: usize) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as IoWrite;
    use tempfile::NamedTempFile;
    use zip::write::SimpleFileOptions;
    use zip::ZipWriter;

    fn create_test_jar(entries: &[(&str, &[u8])]) -> NamedTempFile {
        let file = NamedTempFile::new().unwrap();
        let mut zip = ZipWriter::new(file.reopen().unwrap());
        let options = SimpleFileOptions::default();
        for (name, content) in entries {
            zip.start_file(*name, options).unwrap();
            zip.write_all(content).unwrap();
        }
        zip.finish().unwrap();
        file
    }

    #[test]
    fn analyze_empty_jar() {
        let jar = create_test_jar(&[]);
        let report = analyze_jar(jar.path()).unwrap();
        assert_eq!(report.entry_count, 0);
        assert!(report.categories.is_empty());
        assert!(report.top_packages.is_empty());
        assert!(report.issues.is_empty());
    }

    #[test]
    fn analyze_simple_jar() {
        let jar = create_test_jar(&[
            ("com/example/Main.class", b"fake class bytes"),
            ("META-INF/MANIFEST.MF", b"Manifest-Version: 1.0\n"),
            ("config.edn", b"{:port 8080}"),
        ]);
        let report = analyze_jar(jar.path()).unwrap();
        assert_eq!(report.entry_count, 3);

        let class_cat = report.categories.iter().find(|c| c.name == "Classes");
        assert!(class_cat.is_some());
        assert_eq!(class_cat.unwrap().file_count, 1);

        let meta_cat = report.categories.iter().find(|c| c.name == "Metadata");
        assert!(meta_cat.is_some());
        assert_eq!(meta_cat.unwrap().file_count, 1);

        let res_cat = report.categories.iter().find(|c| c.name == "Resources");
        assert!(res_cat.is_some());
        assert_eq!(res_cat.unwrap().file_count, 1);
    }

    #[test]
    fn analyze_detects_large_resources() {
        let large_data = vec![0u8; 2 * 1024 * 1024]; // 2 MB
        let file = NamedTempFile::new().unwrap();
        let mut zip = ZipWriter::new(file.reopen().unwrap());
        let options = SimpleFileOptions::default();
        zip.start_file("data/model.bin", options).unwrap();
        zip.write_all(&large_data).unwrap();
        zip.finish().unwrap();

        let report = analyze_jar(file.path()).unwrap();
        let large_issue = report
            .issues
            .iter()
            .find(|i| i.message.contains("Large resource"));
        assert!(large_issue.is_some());
        assert!(large_issue.unwrap().message.contains("data/model.bin"));
    }

    #[test]
    fn analyze_shrink_estimate() {
        let jar = create_test_jar(&[
            ("com/example/Main.class", b"class bytes"),
            ("META-INF/maven/com/pom.xml", b"<project/>"),
            ("META-INF/CERT.SF", b"signature"),
            ("com/example/Main.java", b"class Main {}"),
        ]);
        let report = analyze_jar(jar.path()).unwrap();
        assert!(report.shrink_estimate.removable_files > 0);
        assert!(report.shrink_estimate.removable_size > 0);
    }

    #[test]
    fn analyze_clojure_namespaces() {
        let jar = create_test_jar(&[
            ("myapp/core__init.class", b"init class"),
            ("clojure/core__init.class", b"clojure core init"),
        ]);
        let report = analyze_jar(jar.path()).unwrap();
        assert!(!report.clojure_namespaces.is_empty());

        let ns_names: Vec<&str> = report
            .clojure_namespaces
            .iter()
            .map(|n| n.0.as_str())
            .collect();
        assert!(ns_names.contains(&"myapp.core"));
        assert!(ns_names.contains(&"clojure.core"));
    }

    #[test]
    fn analyze_package_grouping() {
        let jar = create_test_jar(&[
            ("org/apache/commons/lang3/StringUtils.class", b"class bytes"),
            ("org/apache/commons/lang3/ArrayUtils.class", b"class bytes"),
            ("com/google/guava/Foo.class", b"class bytes"),
        ]);
        let report = analyze_jar(jar.path()).unwrap();
        assert!(!report.top_packages.is_empty());

        let pkg_names: Vec<&str> = report.top_packages.iter().map(|p| p.0.as_str()).collect();
        assert!(pkg_names.contains(&"org.apache.commons"));
        assert!(pkg_names.contains(&"com.google.guava"));
    }

    #[test]
    fn format_number_with_commas() {
        assert_eq!(format_number(0), "0");
        assert_eq!(format_number(999), "999");
        assert_eq!(format_number(1_000), "1,000");
        assert_eq!(format_number(12_345), "12,345");
        assert_eq!(format_number(1_234_567), "1,234,567");
    }

    #[test]
    fn percentages_use_uncompressed_total() {
        let jar = create_test_jar(&[
            ("com/example/Main.class", b"fake class bytes here"),
            ("config.edn", b"{:port 8080}"),
        ]);
        let report = analyze_jar(jar.path()).unwrap();
        let sum: u64 = report.categories.iter().map(|c| c.size).sum();
        assert_eq!(sum, report.total_uncompressed);
        assert!(report.total_uncompressed > 0);
    }

    #[test]
    fn render_report_does_not_panic() {
        let jar = create_test_jar(&[
            ("com/example/Main.class", b"fake class bytes"),
            ("META-INF/MANIFEST.MF", b"Manifest-Version: 1.0\n"),
        ]);
        let report = analyze_jar(jar.path()).unwrap();
        render_report(&report);
    }
}
