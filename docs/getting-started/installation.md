# Installation

## From Source

Currently, jbundle is installed from source using Cargo (Rust's package manager).

### Prerequisites

* [Rust toolchain](https://rustup.rs/) (1.70+)
* Git

### Steps

```bash
# Clone the repository
git clone https://github.com/avelino/jbundle.git
cd jbundle

# Install globally
cargo install --path .
```

After installation, `jbundle` will be available in your PATH.

### Verify Installation

```bash
jbundle --version
```

## Coming Soon

* Homebrew formula (`brew install jbundle`)
* Pre-built binaries for Linux and macOS
* Cargo crates.io (`cargo install jbundle`)

## Requirements

jbundle itself has no runtime dependencies. However, to **build** applications, you need the appropriate build tools:

| Build System | Required Tool |
|--------------|---------------|
| deps.edn | [Clojure CLI](https://clojure.org/guides/install_clojure) |
| project.clj | [Leiningen](https://leiningen.org/) |
| pom.xml | [Maven](https://maven.apache.org/) |
| build.gradle | [Gradle](https://gradle.org/) |

The **output binary** has no dependencies — it includes everything needed to run.
