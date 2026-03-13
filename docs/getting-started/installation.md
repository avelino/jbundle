# Installation

## Install Script (Recommended)

The fastest way to install jbundle on macOS or Linux:

```bash
curl -sSL https://raw.githubusercontent.com/avelino/jbundle/main/install.sh | sh
```

This detects your platform and downloads the correct pre-compiled binary.

### Custom Install Directory

```bash
JBUNDLE_INSTALL_DIR=~/.local/bin curl -sSL https://raw.githubusercontent.com/avelino/jbundle/main/install.sh | sh
```

### Install a Specific Version

```bash
JBUNDLE_VERSION=v0.1.0 curl -sSL https://raw.githubusercontent.com/avelino/jbundle/main/install.sh | sh
```

## Homebrew

```bash
brew tap avelino/jbundle
brew install jbundle
```

Works on macOS (Intel and Apple Silicon) and Linux via Linuxbrew.

## Pre-compiled Binaries

Download from [GitHub Releases](https://github.com/avelino/jbundle/releases):

| Platform | Binary |
|----------|--------|
| Linux x86_64 | `jbundle-linux-x86_64.tar.gz` |
| Linux ARM64 | `jbundle-linux-aarch64.tar.gz` |
| macOS x86_64 | `jbundle-darwin-x86_64.tar.gz` |
| macOS ARM64 | `jbundle-darwin-aarch64.tar.gz` |

```bash
# Example: manual install on Linux x86_64
curl -sSL https://github.com/avelino/jbundle/releases/latest/download/jbundle-linux-x86_64.tar.gz | tar xz
sudo mv jbundle /usr/local/bin/
```

## From Source

Build from source using Cargo (Rust's package manager).

### Prerequisites

* [Rust toolchain](https://rustup.rs/) (1.70+)
* Git
* SSL development libraries
  * Debian/Ubuntu: `sudo apt update && sudo apt install libssl-dev`

### Steps

```bash
git clone https://github.com/avelino/jbundle.git
cd jbundle
cargo install --path .
```

## Verify Installation

```bash
jbundle --version
```

## Requirements

jbundle itself has no runtime dependencies. However, to **build** applications, you need the appropriate build tools:

| Build System | Required Tool |
|--------------|---------------|
| deps.edn | [Clojure CLI](https://clojure.org/guides/install_clojure) |
| project.clj | [Leiningen](https://leiningen.org/) |
| pom.xml | [Maven](https://maven.apache.org/) |
| build.gradle | [Gradle](https://gradle.org/) |

The **output binary** has no dependencies — it includes everything needed to run.
