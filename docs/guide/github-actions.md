# GitHub Actions

Build self-contained JVM binaries in your CI/CD pipeline.

## Quick Start (Copy & Paste)

Copy this workflow to `.github/workflows/build.yml`:

```yaml
name: Build Binary

on:
  push:
    branches: [main]
  pull_request:

jobs:
  build:
    runs-on: ubuntu-22.04

    steps:
      - uses: actions/checkout@v4

      - name: Set up JDK
        uses: actions/setup-java@v4
        with:
          distribution: temurin
          java-version: 21

      - name: Cache jbundle
        uses: actions/cache@v4
        with:
          path: ~/.jbundle/cache
          key: jbundle-linux-x64

      - name: Install jbundle
        uses: baptiste0928/cargo-install@v3
        with:
          crate: jbundle
          git: https://github.com/avelino/jbundle
          branch: main

      - name: Build binary
        run: jbundle build --input . --output ./dist/myapp

      - name: Test binary
        run: ./dist/myapp --help

      - name: Upload artifact
        uses: actions/upload-artifact@v4
        with:
          name: myapp-linux-x64
          path: ./dist/myapp
```

**What to change:**

- `myapp` → your application name
- `java-version: 21` → your Java version

---

## Installation

Install jbundle from source using `cargo-install`:

```yaml
- name: Install jbundle
  uses: baptiste0928/cargo-install@v3
  with:
    crate: jbundle
    git: https://github.com/avelino/jbundle/
    branch: main
```

> **Note:** jbundle will be published to crates.io soon, simplifying installation to just `crate: jbundle`.

## Basic Usage

```yaml
- name: Build binary
  run: jbundle build --input . --output ./dist/app
```

## Complete Workflow Example

```yaml
name: Build Binaries

on:
  push:
    branches: [main]
  pull_request:

jobs:
  build:
    runs-on: ubuntu-22.04

    steps:
      - uses: actions/checkout@v4

      - name: Set up JDK
        uses: actions/setup-java@v4
        with:
          distribution: temurin
          java-version: 21

      - name: Install jbundle
        uses: baptiste0928/cargo-install@v3
        with:
          crate: jbundle
          git: https://github.com/avelino/jbundle/
          branch: main

      - name: Build binary
        run: jbundle build --input . --output ./dist/myapp

      - name: Upload artifact
        uses: actions/upload-artifact@v4
        with:
          name: myapp-linux-x64
          path: ./dist/myapp
```

## Gradle Multi-Project

For projects with multiple subprojects (like JabRef):

```yaml
- name: Build with Gradle first
  run: ./gradlew :jabgui:jlinkZip

- name: Build binary with jbundle
  run: |
    mkdir -p build/jbundle
    jbundle build \
      --input . \
      --gradle-project jabgui \
      --jlink-runtime jabgui/build/packages \
      --output ./build/jbundle/jabgui

- name: Smoke test
  run: build/jbundle/jabgui --help

- name: Upload artifact
  uses: actions/upload-artifact@v4
  with:
    name: jbundle-${{ matrix.displayName }}
    path: build/jbundle/jabgui*
```

## Cross-Platform Builds

Build for multiple platforms using a matrix:

```yaml
jobs:
  build:
    strategy:
      matrix:
        include:
          - os: ubuntu-22.04
            target: linux-x64
          - os: ubuntu-22.04
            target: linux-aarch64
          - os: macos-14
            target: macos-aarch64
          - os: macos-13
            target: macos-x64

    runs-on: ${{ matrix.os }}

    steps:
      - uses: actions/checkout@v4

      - name: Set up JDK
        uses: actions/setup-java@v4
        with:
          distribution: temurin
          java-version: 21

      - name: Install jbundle
        uses: baptiste0928/cargo-install@v3
        with:
          crate: jbundle
          git: https://github.com/avelino/jbundle/
          branch: main

      - name: Build binary
        run: |
          jbundle build \
            --input . \
            --output ./dist/myapp-${{ matrix.target }} \
            --target ${{ matrix.target }}

      - name: Upload artifact
        uses: actions/upload-artifact@v4
        with:
          name: myapp-${{ matrix.target }}
          path: ./dist/myapp-${{ matrix.target }}
```

> **Note:** Cross-compilation (e.g., building `linux-aarch64` on `ubuntu-22.04`) works for the jlink runtime but requires the target JDK to be available.

## Caching

Speed up builds by caching the jbundle cache directory:

```yaml
- name: Cache jbundle
  uses: actions/cache@v4
  with:
    path: ~/.jbundle/cache
    key: jbundle-${{ runner.os }}-${{ hashFiles('**/jbundle.toml') }}
    restore-keys: |
      jbundle-${{ runner.os }}-
```

This caches downloaded JDKs and extracted runtimes.

## Using jbundle.toml

Instead of passing flags, use a config file:

```toml
# jbundle.toml
java_version = 21
profile = "cli"
jvm_args = ["-Xmx512m"]
```

Then your workflow simplifies to:

```yaml
- name: Build binary
  run: jbundle build --input . --output ./dist/myapp
```

## Environment Variables

Enable debug logging for troubleshooting:

```yaml
- name: Build binary (debug)
  run: jbundle build --input . --output ./dist/myapp
  env:
    RUST_LOG: debug
```

## Release Workflow

Create releases with binaries for all platforms:

```yaml
name: Release

on:
  push:
    tags:
      - 'v*'

jobs:
  build:
    strategy:
      matrix:
        include:
          - os: ubuntu-22.04
            target: linux-x64
          - os: macos-14
            target: macos-aarch64

    runs-on: ${{ matrix.os }}

    steps:
      - uses: actions/checkout@v4

      - name: Set up JDK
        uses: actions/setup-java@v4
        with:
          distribution: temurin
          java-version: 21

      - name: Install jbundle
        uses: baptiste0928/cargo-install@v3
        with:
          crate: jbundle
          git: https://github.com/avelino/jbundle/
          branch: main

      - name: Build binary
        run: |
          jbundle build \
            --input . \
            --output ./myapp-${{ matrix.target }} \
            --target ${{ matrix.target }}

      - name: Upload to release
        uses: softprops/action-gh-release@v1
        with:
          files: ./myapp-${{ matrix.target }}
```

## Troubleshooting

### Build hangs at "Detecting build system"

This usually means Gradle is downloading dependencies. Add Gradle caching:

```yaml
- name: Cache Gradle
  uses: actions/cache@v4
  with:
    path: |
      ~/.gradle/caches
      ~/.gradle/wrapper
    key: gradle-${{ runner.os }}-${{ hashFiles('**/*.gradle*', '**/gradle-wrapper.properties') }}
```

### Out of memory

Increase JVM memory in your config:

```toml
# jbundle.toml
jvm_args = ["-Xmx2g"]
```

Or via CLI:

```yaml
- name: Build binary
  run: jbundle build --input . --output ./dist/myapp --jvm-args "-Xmx2g"
```
