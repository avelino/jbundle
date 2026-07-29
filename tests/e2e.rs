//! End-to-end harness: build a real self-contained binary from a fixture JAR,
//! run it, and compare stdout + exit code against a golden expectation.
//!
//! This exercises the fragile part of the pipeline that unit tests don't cover:
//! the self-extracting stub, extraction into `~/.jbundle/cache/`, the jlink
//! runtime, and actual execution of the produced binary.
//!
//! It requires a full JDK (javac, jar, jlink, jdeps, java) on the host. When no
//! usable JDK is found, each test prints a notice and passes — so `cargo test`
//! stays green on machines without a JVM toolchain. In CI (with setup-java),
//! set `JBUNDLE_E2E=1` to turn a missing/incomplete JDK into a hard failure and
//! guarantee the harness actually ran.

use std::path::{Path, PathBuf};
use std::process::Command;

/// A golden test case: a Java program and its expected observable behavior.
struct Case {
    /// `Hello`-style class name (must match the public class in `source`).
    class: &'static str,
    source: &'static str,
    expected_stdout: &'static str,
    expected_exit: i32,
    /// Whether to pass `--shrink` (covers the shrink -> pack path).
    shrink: bool,
}

const EXE_SUFFIX: &str = if cfg!(windows) { ".exe" } else { "" };

/// Locate a JDK home that has the tools we need. Order: `$JAVA_HOME`, then the
/// parent of a `javac` found on `PATH`. Returns `None` if nothing usable exists.
fn find_jdk() -> Option<PathBuf> {
    if let Ok(jh) = std::env::var("JAVA_HOME") {
        let home = PathBuf::from(jh);
        if jdk_is_complete(&home) {
            return Some(home);
        }
    }

    // On macOS the tools in /usr/bin are shims; the real JDK home comes from
    // `/usr/libexec/java_home`. Resolving `javac` on PATH would give `/usr`.
    if cfg!(target_os = "macos") {
        if let Ok(out) = Command::new("/usr/libexec/java_home").output() {
            if out.status.success() {
                let home = PathBuf::from(String::from_utf8_lossy(&out.stdout).trim());
                if jdk_is_complete(&home) {
                    return Some(home);
                }
            }
        }
    }

    // `javac` on PATH -> <home>/bin/javac -> home is two levels up.
    let javac = which_on_path(&format!("javac{EXE_SUFFIX}"))?;
    let home = javac.parent()?.parent()?.to_path_buf();
    if jdk_is_complete(&home) {
        return Some(home);
    }
    None
}

/// A JDK is usable for this harness only if every tool the pipeline needs is
/// present (a JRE, which lacks javac/jlink/jdeps, is not enough).
fn jdk_is_complete(home: &Path) -> bool {
    ["javac", "jar", "jlink", "jdeps", "java"]
        .iter()
        .all(|tool| {
            home.join("bin")
                .join(format!("{tool}{EXE_SUFFIX}"))
                .is_file()
        })
}

fn which_on_path(exe: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(exe))
        .find(|candidate| candidate.is_file())
}

fn tool(home: &Path, name: &str) -> PathBuf {
    home.join("bin").join(format!("{name}{EXE_SUFFIX}"))
}

/// Skip the whole harness when there's no JDK, unless `JBUNDLE_E2E=1` forces it.
/// Returns `None` to signal "skip"; the caller returns early on `None`.
fn require_jdk() -> Option<PathBuf> {
    match find_jdk() {
        Some(home) => Some(home),
        None => {
            let msg = "e2e: no complete JDK found (need javac/jar/jlink/jdeps/java); skipping";
            if std::env::var("JBUNDLE_E2E").as_deref() == Ok("1") {
                panic!("{msg} — but JBUNDLE_E2E=1 requires it");
            }
            eprintln!("{msg}");
            None
        }
    }
}

/// Compile `case.source` into a runnable JAR with a `Main-Class` manifest.
fn build_fixture_jar(jdk: &Path, work: &Path, case: &Case) -> PathBuf {
    let src = work.join(format!("{}.java", case.class));
    std::fs::write(&src, case.source).expect("write java source");

    let status = Command::new(tool(jdk, "javac"))
        .arg("-d")
        .arg(work)
        .arg(&src)
        .status()
        .expect("run javac");
    assert!(status.success(), "javac failed for {}", case.class);

    // `jar --create --file out.jar --main-class Hello -C <dir> Hello.class`
    let jar_path = work.join(format!("{}.jar", case.class));
    let status = Command::new(tool(jdk, "jar"))
        .arg("--create")
        .arg("--file")
        .arg(&jar_path)
        .arg("--main-class")
        .arg(case.class)
        .arg("-C")
        .arg(work)
        .arg(format!("{}.class", case.class))
        .status()
        .expect("run jar");
    assert!(status.success(), "jar failed for {}", case.class);

    jar_path
}

/// Full pipeline for one case: fixture JAR -> jbundle binary -> run -> assert.
fn run_case(case: &Case) {
    let Some(jdk) = require_jdk() else { return };

    let work = tempfile::tempdir().expect("tempdir");
    let jar = build_fixture_jar(&jdk, work.path(), case);
    let out_bin = work.path().join(format!("{}-app{EXE_SUFFIX}", case.class));

    // Build the self-contained binary. `--java-home` reuses the local JDK so the
    // test never hits the network (Adoptium download) and stays hermetic.
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_jbundle"));
    cmd.arg("build")
        .arg("--input")
        .arg(&jar)
        .arg("--output")
        .arg(&out_bin)
        .arg("--java-home")
        .arg(&jdk)
        .arg("--no-appcds"); // keep first run deterministic and fast
    if case.shrink {
        cmd.arg("--shrink");
    }

    let build = cmd.output().expect("run jbundle build");
    assert!(
        build.status.success(),
        "jbundle build failed for {} (shrink={}):\n--- stdout ---\n{}\n--- stderr ---\n{}",
        case.class,
        case.shrink,
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr),
    );
    assert!(
        out_bin.is_file(),
        "expected output binary at {}",
        out_bin.display()
    );

    // Run the produced binary and capture observable behavior.
    let run = Command::new(&out_bin)
        .output()
        .expect("run produced binary");

    let stdout = String::from_utf8_lossy(&run.stdout);
    // Normalize trailing whitespace like the reference harness does.
    assert_eq!(
        stdout.trim_end(),
        case.expected_stdout,
        "stdout mismatch for {} (shrink={})\nstderr:\n{}",
        case.class,
        case.shrink,
        String::from_utf8_lossy(&run.stderr),
    );
    assert_eq!(
        run.status.code(),
        Some(case.expected_exit),
        "exit code mismatch for {} (shrink={})",
        case.class,
        case.shrink,
    );
}

#[test]
fn hello_world_with_shrink() {
    run_case(&Case {
        class: "HelloShrink",
        source: r#"
public class HelloShrink {
    public static void main(String[] args) {
        System.out.println("hello from jbundle e2e");
    }
}
"#,
        expected_stdout: "hello from jbundle e2e",
        expected_exit: 0,
        shrink: true,
    });
}

#[test]
fn custom_exit_code() {
    run_case(&Case {
        class: "ExitFortyTwo",
        source: r#"
public class ExitFortyTwo {
    public static void main(String[] args) {
        System.out.print("bye");
        System.exit(42);
    }
}
"#,
        expected_stdout: "bye",
        expected_exit: 42,
        shrink: false,
    });
}
