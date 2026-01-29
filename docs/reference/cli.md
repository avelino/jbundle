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
| `--shrink` | — | Shrink uberjar by removing non-essential files |
| `--no-appcds` | — | Disable AppCDS generation |
| `--crac` | — | Enable CRaC checkpoint (Linux only) |
| `--gradle-project <NAME>` | — | Gradle subproject to build (multi-project) |
| `--all` | — | Build all application subprojects (Gradle) |
| `--modules <LIST>` | — | Manual module list, comma-separated |
| `--jlink-runtime <PATH>` | — | Path to existing jlink runtime to reuse |
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

# Multiple JVM arguments
jbundle build --input . --output ./app --jvm-args "-Xmx512m -XX:+UseZGC"

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
