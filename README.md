# clj-pack

Package JVM applications (Clojure, Java) into self-contained binaries. No JVM installation required to run the output.

```
project/jar → clj-pack → single binary (runs anywhere)
```

## Why?

Deploying JVM apps usually means shipping a JAR and requiring a JVM on the target machine. The common solution is GraalVM native-image, but it brings its own set of problems: long compilation times, reflection configuration headaches, incompatible libraries, and a complex toolchain.

`clj-pack` is a practical alternative. It bundles a minimal JVM runtime (via `jlink`) with your uberjar into a single executable — zero external dependencies, everything included in the binary. No GraalVM setup, no reflection configs, no library compatibility issues. Your app runs exactly as it does in development.

The result: one file, zero runtime dependencies, full JVM compatibility, instant startup on second run.

### clj-pack vs GraalVM native-image

| | clj-pack | GraalVM native-image |
|---|---|---|
| Compatibility | 100% JVM compatible | Requires reflection config, some libs unsupported |
| Build time | Fast (jlink + packaging) | Slow (ahead-of-time compilation) |
| Binary size | ~30-50 MB | ~20-40 MB |
| First run | Extracts once, cached | Instant |
| Setup | Just `clj-pack` | GraalVM + native-image + config |
| Debugging | Standard JVM tooling | Limited |

## Quick Start

```sh
# Build from a Clojure project (deps.edn or project.clj)
clj-pack build --input ./my-clojure-app --output ./dist/my-app

# Build from a Java project (pom.xml or build.gradle)
clj-pack build --input ./my-java-app --output ./dist/my-app

# Build from a pre-built JAR
clj-pack build --input ./target/app.jar --output ./dist/my-app

# Run it — no Java needed on the system
./dist/my-app
```

## How It Works

1. Detect build system (deps.edn, project.clj, pom.xml, or build.gradle)
2. Build JAR (clojure / lein / mvn / gradle)
3. Download JDK from Adoptium (cached locally)
4. Detect modules with jdeps
5. Create minimal runtime with jlink (~30-50 MB)
6. Pack runtime + JAR into self-contained binary

The generated binary is a shell stub + tar.gz payload. On first execution it extracts to `~/.clj-pack/cache/` (cached by content hash), then runs `java -jar` from the minimal runtime. Subsequent runs skip extraction entirely.

## Installation

### From source

```sh
git clone https://github.com/avelino/clj-pack.git
cd clj-pack
cargo install --path .
```

## Usage

```sh
# Build with specific Java version
clj-pack build --input . --output ./dist/app --java-version 21

# Cross-platform target
clj-pack build --input . --output ./dist/app --target linux-x64

# Pass JVM arguments
clj-pack build --input . --output ./dist/app --jvm-args "-Xmx512m"

# Show cache info
clj-pack info

# Clean cache
clj-pack clean
```

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
