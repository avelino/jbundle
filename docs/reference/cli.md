# CLI Commands

Complete reference for jbundle command-line interface.

## jbundle build

Build a self-contained binary from a JVM project or JAR.

```bash
jbundle build [OPTIONS] --input <PATH> --output <PATH>
```

### Required Arguments

| Argument | Description |
|----------|-------------|
| `--input <PATH>` | Project directory or JAR file |
| `--output <PATH>` | Output binary path |

### Options

| Option | Default | Description |
|--------|---------|-------------|
| `--java-version <N>` | `21` | JDK version to bundle (11, 17, 21, 22, 23, 24, 25) |
| `--target <TARGET>` | current | Target platform (see [Platforms](platforms.md)) |
| `--profile <PROFILE>` | `server` | JVM profile (`cli` or `server`) |
| `--jvm-args <ARGS>` | — | JVM arguments (e.g., `-Xmx512m`) |
| `--shrink [true\|false]` | `false` | Shrink uberjar by removing non-essential files |
| `--no-appcds` | — | Disable AppCDS generation |
| `--crac` | — | Enable CRaC checkpoint (Linux only) |
| `--gradle-project <NAME>` | — | Gradle subproject to build (multi-project) |
| `--all` | — | Build all application subprojects (Gradle) |
| `--modules <LIST>` | — | Manual module list, comma-separated |
| `--jlink-runtime <PATH>` | — | Path to existing jlink runtime to reuse (must contain `bin/java`) |
| `-v, --verbose` | — | Enable verbose output |

### Examples

```bash
# Basic build
jbundle build --input ./my-app --output ./dist/app

# With Java 17
jbundle build --input . --output ./app --java-version 17

# CLI profile for fast startup
jbundle build --input . --output ./app --profile cli

# Cross-compile for Linux
jbundle build --input . --output ./app --target linux-x64

# Multiple JVM arguments (use server profile with custom GC)
jbundle build --input . --output ./app --profile server --jvm-args "-Xmx512m -XX:+UseZGC"

# Shrink the uberjar (remove non-essential files)
jbundle build --input . --output ./app --shrink

# Explicitly disable shrinking
jbundle build --input . --output ./app --shrink false

# From pre-built JAR
jbundle build --input ./target/app.jar --output ./dist/app

# With CRaC (Linux)
jbundle build --input . --output ./app --crac

# Gradle multi-project: specific subproject
jbundle build --input . --output ./dist/app --gradle-project app

# Gradle multi-project: build all
jbundle build --input . --output ./dist --all

# Manual module specification
jbundle build --input . --output ./app --modules java.base,java.sql,java.logging

# Reuse existing jlink runtime
jbundle build --input . --output ./app --jlink-runtime ./build/jlink
```

## jbundle analyze

Analyze a JAR or project and report size breakdown, top dependencies, and potential issues.

```bash
jbundle analyze [OPTIONS]
```

### Options

| Option | Default | Description |
|--------|---------|-------------|
| `--input <PATH>` | `.` | Project directory or pre-built JAR file |

When given a project directory, jbundle detects the build system, builds the uberjar, then analyzes it. When given a JAR file directly, it skips the build step.

### Output

The report includes:

* **Category breakdown** — Classes, Resources, Native libs, Metadata, Clojure/Java sources with size and file count
* **Top packages by size** — Grouped by first 3 path segments (e.g., `org.apache.commons`)
* **Clojure namespaces** — Detected from `__init.class` entries
* **Shrink estimate** — How much `--shrink` would save
* **Potential issues** — Duplicate classes, large resources (> 1 MB)

### Examples

```bash
# Analyze current project
jbundle analyze

# Analyze a specific project
jbundle analyze --input ./my-app

# Analyze a pre-built JAR
jbundle analyze --input ./target/app-standalone.jar
```

## jbundle info

Display cache information.

```bash
jbundle info
```

Shows:
* Cached JDK downloads
* Extracted runtimes
* Application caches
* Total cache size

## jbundle clean

Remove all cached data.

```bash
jbundle clean
```

Removes everything in `~/.jbundle/cache/`.

## jbundle --version

Print version information.

```bash
jbundle --version
```

## jbundle --help

Print help message.

```bash
jbundle --help
jbundle build --help
```

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | Success |
| `1` | Build error (compilation failed, JAR not found, etc.) |
| `2` | Configuration error (invalid arguments, missing input) |

## Environment Variables

| Variable | Description |
|----------|-------------|
| `RUST_LOG` | Logging level (`error`, `warn`, `info`, `debug`, `trace`) |

### Logging Examples

```bash
# Show debug output
RUST_LOG=debug jbundle build --input . --output ./app

# Show only errors
RUST_LOG=error jbundle build --input . --output ./app

# Verbose trace logging
RUST_LOG=trace jbundle build --input . --output ./app
```
