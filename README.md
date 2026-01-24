# jbundle

Package JVM applications (Clojure, Java, Kotlin, Scala, Groovy — any JVM language) into self-contained binaries. No JVM installation required to run the output.

> **Note:** Previously known as `clj-pack`. Renamed to `jbundle` to reflect support for all JVM languages, not just Clojure.

```
project/jar → jbundle → single binary (runs anywhere)
```

## Why?

Deploying JVM apps usually means shipping a JAR and requiring a JVM on the target machine. The common solution is GraalVM native-image, but it brings its own set of problems: long compilation times, reflection configuration headaches, incompatible libraries, and a complex toolchain.

`jbundle` is a practical alternative. It bundles a minimal JVM runtime (via `jlink`) with your uberjar into a single executable — zero external dependencies, everything included in the binary. No GraalVM setup, no reflection configs, no library compatibility issues. Your app runs exactly as it does in development.

The result: one file, zero runtime dependencies, full JVM compatibility, instant startup on second run.

### jbundle vs GraalVM native-image

| | jbundle | GraalVM native-image |
|---|---|---|
| Compatibility | 100% JVM compatible | Requires reflection config, some libs unsupported |
| Build time | Fast (jlink + packaging) | Slow (ahead-of-time compilation) |
| Binary size | ~30-50 MB | ~20-40 MB |
| Startup (warm) | ~200-350ms (AppCDS) / ~10-50ms (CRaC) | ~10-50ms |
| First run | Extracts + generates CDS (~2-5s extra), cached | Instant |
| Setup | Just `jbundle` | GraalVM + native-image + config |
| Debugging | Standard JVM tooling | Limited |

## Quick Start

```sh
# Build from a Clojure project (deps.edn or project.clj)
jbundle build --input ./my-clojure-app --output ./dist/my-app

# Build from a Java project (pom.xml or build.gradle)
jbundle build --input ./my-java-app --output ./dist/my-app

# Build from a pre-built JAR
jbundle build --input ./target/app.jar --output ./dist/my-app

# Run it — no Java needed on the system
./dist/my-app
```

## How It Works

1. Detect build system (deps.edn, project.clj, pom.xml, or build.gradle)
2. Build JAR (clojure / lein / mvn / gradle)
3. Download JDK from Adoptium (cached locally)
4. Detect modules with jdeps
5. Create minimal runtime with jlink (~30-50 MB)
6. Create CRaC checkpoint for instant restore (optional, Linux only)
7. Pack into multi-layer binary (runtime + compressed app.jar)

The generated binary uses a multi-layer format: `[stub] [runtime.tar.gz] [app.jar.gz] [crac.tar.gz?]`. Each layer is cached independently by content hash under `~/.jbundle/cache/`. Updating only your app code reuses the cached runtime — no re-extraction needed.

## Build Error Diagnostics

When a build fails, jbundle parses the compiler output and displays structured diagnostics with source context — similar to `rustc`:

```
error: Unable to resolve symbol: prntln
 --> src/example/core.clj:9:5
   |
 7 | (defn process-data [data]
 8 |   (let [result (map inc data)]
 9 |     (prntln "Processing:" result)
   |     ^^^^^^^ symbol not found
10 |     (reduce + result)))
   |
```

Supported for all build systems (Clojure, Maven, Gradle/Kotlin). Falls back to raw error output when the format is not recognized.

## Startup Performance

jbundle pursues GraalVM-level startup times without requiring AOT compilation. It combines multiple HotSpot-native techniques to make the **first run slightly slower** but **all subsequent runs significantly faster**.

### First Run vs Subsequent Runs

| | First run | Subsequent runs |
|---|---|---|
| What happens | Extracts runtime + app, JVM generates AppCDS archive | Everything cached, JVM loads pre-built class metadata |
| Overhead | +2-5s (extraction + CDS generation) | None |
| Startup (cli profile) | ~800-1500ms | ~200-350ms (**~60-75% faster**) |
| Startup (server profile) | ~1000-2000ms | ~400-600ms (**~50-70% faster**) |
| Startup (CRaC restore) | ~800-1500ms | ~10-50ms (**~95% faster**) |

**Why the first run is slower:** The JVM needs to extract the compressed layers (runtime + app.jar) from the binary, then uses `-XX:+AutoCreateSharedArchive` to analyze which classes are loaded during startup and generates a shared archive (`.jsa` file). This is a one-time cost — the archive is cached alongside the app and reused on every subsequent execution.

**Why subsequent runs are faster:** The JVM skips class file parsing, verification, and layout computation by loading pre-processed class metadata directly from the `.jsa` archive. Combined with profile-specific flags (C1-only compilation, SerialGC for CLI tools), startup overhead is reduced to the minimum the HotSpot JVM can achieve without AOT compilation.

### JVM Profiles

The `--profile` flag selects JVM flags tuned for different workloads:

- **`server`** (default) — no extra flags, standard HotSpot behavior. Best for long-running services.
- **`cli`** — tiered compilation at level 1 only + SerialGC. Optimized for short-lived CLI tools (~200-350ms startup after first run).

### AppCDS (Class Data Sharing)

Enabled by default (JDK 19+). On first execution, the JVM automatically generates a shared archive (`.jsa`) containing pre-processed class metadata for your application. Subsequent runs load this archive directly, skipping class file parsing and verification.

The archive is stored at `~/.jbundle/cache/app-<hash>/app.jsa` and is tied to the specific app version. A new app version triggers a new archive generation on its first run.

Disable with `--no-appcds` if you observe issues with class loading or want to minimize first-run overhead.

### CRaC (Coordinated Restore at Checkpoint)

Optional (`--crac`). On supported JDKs, jbundle creates a checkpoint of the running app after warmup. On subsequent runs, the JVM restores from checkpoint instead of starting from scratch — achieving ~10-50ms startup, comparable to native binaries.

Requires a CRaC-enabled JDK (e.g., Azul Zulu with CRaC). Falls back to AppCDS + profile flags if restore fails. Linux only.

### Layered Cache

The binary is composed of independent layers, each cached by content hash:

```
~/.jbundle/cache/
  rt-<hash>/       # JVM runtime (reused across app rebuilds)
  app-<hash>/      # app.jar + app.jsa (generated on first run)
  crac-<hash>/     # CRaC checkpoint (if enabled)
```

Changing only your application code does not re-extract the runtime. This matters for CI/CD and containers where the runtime layer (~30-50 MB) stays warm across deploys.

## Installation

### From source

```sh
git clone https://github.com/avelino/jbundle.git
cd jbundle
cargo install --path .
```

## Usage

```sh
# Build with specific Java version
jbundle build --input . --output ./dist/app --java-version 21

# Cross-platform target
jbundle build --input . --output ./dist/app --target linux-x64

# Pass JVM arguments
jbundle build --input . --output ./dist/app --jvm-args "-Xmx512m"

# CLI profile (fast startup, optimized for short-lived tools)
jbundle build --input . --output ./dist/app --profile cli

# Disable AppCDS generation
jbundle build --input . --output ./dist/app --no-appcds

# Enable CRaC checkpoint (Linux, requires CRaC-enabled JDK)
jbundle build --input . --output ./dist/app --crac

# Show cache info
jbundle info

# Clean cache
jbundle clean
```

### Configuration file

You can create an optional `jbundle.toml` in your project root to avoid repeating CLI flags:

```toml
# jbundle.toml
java_version = 21
target = "linux-x64"
shrink = true
jvm_args = ["-Xmx512m", "-XX:+UseZGC"]
profile = "cli"       # "cli" or "server" (default: "server")
appcds = true         # generate AppCDS archive (default: true)
crac = false          # enable CRaC checkpoint (default: false)
```

All fields are optional. CLI flags always take precedence over the config file:

```
CLI flags > jbundle.toml > built-in defaults
```

### Supported JDK versions

jbundle downloads JDK runtimes from [Adoptium](https://adoptium.net/). The `java_version` field (or `--java-version` flag) accepts the following versions:

| Version | Type    | Status         |
| ------- | ------- | -------------- |
| `11`    | LTS     | Supported      |
| `17`    | LTS     | Supported      |
| `21`    | **LTS** | **Default**    |
| `22`    | STS     | Supported      |
| `23`    | STS     | Supported      |
| `24`    | STS     | Supported      |
| `25`    | LTS     | Supported      |

LTS (Long-Term Support) versions are recommended for production. The default is `21` when not specified and not auto-detected from the JAR.

> **Note:** Java 8 is not supported because jbundle relies on `jlink` and `jdeps`, which were introduced in Java 9.

### Supported platforms

| Target          | Status    |
| --------------- | --------- |
| `linux-x64`     | Supported |
| `linux-aarch64` | Supported |
| `macos-x64`     | Supported |
| `macos-aarch64` | Supported |

### Supported build systems

| System                  | Detection                          |
| ----------------------- | ---------------------------------- |
| deps.edn (tools.build)  | `deps.edn` in project root        |
| Leiningen               | `project.clj` in project root     |
| Maven                   | `pom.xml` in project root          |
| Gradle                  | `build.gradle(.kts)` in project root |

## Contributing

Contributions are welcome. Here's how to get started:

1. Fork the repository
2. Create a branch for your change
3. Make your changes with tests if applicable
4. Open a pull request

### Development

```sh
# Build
cargo build

# Run against example projects
cargo run -- build --input ./example/clojure-deps --output ./dist/app
cargo run -- build --input ./example/clojure-lein --output ./dist/app
cargo run -- build --input ./example/java-pom --output ./dist/app
cargo run -- build --input ./example/java-gradle --output ./dist/app

# Or after installing
jbundle build --input ./example/clojure-deps --output ./dist/app

# Run the generated binary
./dist/app
```

### Ideas for contribution

- Windows support
- Custom `jlink` module list override
- Compression options (zstd, xz)
- CI/CD integration examples
- Homebrew formula

## License

MIT
