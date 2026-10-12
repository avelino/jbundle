# GitHub Actions

Build self-contained JVM binaries in your CI/CD pipeline.

## Quick Start

```yaml
name: Build Binary

on:
  push:
    branches: [main]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - uses: actions/setup-java@v4
        with:
          distribution: temurin
          java-version: 21

      - uses: avelino/jbundle@main
        with:
          input: .
          output: ./dist/myapp

      - uses: actions/upload-artifact@v4
        with:
          name: myapp-linux-x64
          path: ./dist/myapp
```

That's it. Replace `myapp` with your app name.

## Action Reference

### Inputs

| Input | Required | Default | Description |
|-------|----------|---------|-------------|
| `version` | no | `latest` | jbundle version to install (e.g., `v0.2.0`) |
| `input` | no | — | Path to project directory or JAR file |
| `output` | no | — | Output binary path |
| `java-version` | no | — | JDK version to bundle (11, 17, 21, etc.) |
| `target` | no | — | Target platform (`linux-x64`, `macos-aarch64`, etc.) |
| `profile` | no | — | JVM profile (`cli` or `server`) |
| `shrink` | no | — | Shrink uberjar (`true` to enable) |
| `args` | no | — | Additional arguments passed to `jbundle build` |
| `install-only` | no | `false` | Only install jbundle, don't run build |

### Outputs

| Output | Description |
|--------|-------------|
| `binary` | Path to the output binary |
| `version` | Installed jbundle version |

## Examples

### CLI Tool with Fast Startup

```yaml
- uses: avelino/jbundle@main
  with:
    input: .
    output: ./dist/mycli
    profile: cli
    shrink: true
```

### Specific Java Version

```yaml
- uses: avelino/jbundle@main
  with:
    input: .
    output: ./dist/myapp
    java-version: 17
```

### From Pre-built JAR

```yaml
- name: Build JAR
  run: ./gradlew shadowJar

- uses: avelino/jbundle@main
  with:
    input: ./build/libs/app-all.jar
    output: ./dist/myapp
```

### Install Only (Custom Build Commands)

```yaml
- uses: avelino/jbundle@main
  with:
    install-only: true

- name: Build with custom flags
  run: |
    jbundle build \
      --input . \
      --output ./dist/myapp \
      --modules java.base,java.sql,java.desktop \
      --jvm-args "-Xmx1g"
```

### Extra Arguments

```yaml
- uses: avelino/jbundle@main
  with:
    input: .
    output: ./dist/myapp
    args: "--gradle-project app --build-args '-PeaJdkBuild=false'"
```

### Pin to a Specific Version

```yaml
- uses: avelino/jbundle@v0.2.0
  with:
    input: .
    output: ./dist/myapp
```

## Cross-Platform Builds

Build for multiple platforms using a matrix:

```yaml
jobs:
  build:
    strategy:
      matrix:
        include:
          - os: ubuntu-latest
            target: linux-x64
          - os: macos-14
            target: macos-aarch64
          - os: macos-13
            target: macos-x64

    runs-on: ${{ matrix.os }}

    steps:
      - uses: actions/checkout@v4

      - uses: actions/setup-java@v4
        with:
          distribution: temurin
          java-version: 21

      - uses: avelino/jbundle@main
        with:
          input: .
          output: ./dist/myapp-${{ matrix.target }}
          target: ${{ matrix.target }}

      - uses: actions/upload-artifact@v4
        with:
          name: myapp-${{ matrix.target }}
          path: ./dist/myapp-${{ matrix.target }}
```

## Windows CI

jbundle works on `windows-latest` runners. Build binaries for all platforms from Windows:

```yaml
jobs:
  build:
    strategy:
      matrix:
        include:
          - os: ubuntu-latest
            target: linux-x64
          - os: macos-14
            target: macos-aarch64
          - os: windows-latest
            target: linux-x64

    runs-on: ${{ matrix.os }}

    steps:
      - uses: actions/checkout@v4

      - uses: actions/setup-java@v4
        with:
          distribution: temurin
          java-version: 21

      - uses: avelino/jbundle@main
        with:
          input: .
          output: ./dist/myapp-${{ matrix.target }}
          target: ${{ matrix.target }}

      - uses: actions/upload-artifact@v4
        with:
          name: myapp-${{ matrix.os }}-${{ matrix.target }}
          path: ./dist/myapp-${{ matrix.target }}
```

> **Note:** The output binary uses a Unix shell stub, so `--target` must be a Linux or macOS platform. Windows is supported as a build host, not an output target.

## Gradle Multi-Project

For projects with multiple subprojects (like JabRef):

```yaml
- uses: avelino/jbundle@main
  with:
    input: .
    output: ./dist/myapp
    args: "--gradle-project app"
```

Or with a pre-built jlink runtime:

```yaml
- name: Build with Gradle
  run: ./gradlew :jabgui:jlinkZip

- uses: avelino/jbundle@main
  with:
    input: .
    output: ./dist/jabgui
    args: "--gradle-project jabgui --jlink-runtime jabgui/build/packages"
```

## Caching

Speed up builds by caching the jbundle JDK cache:

```yaml
- uses: actions/cache@v4
  with:
    path: ~/.jbundle/cache
    key: jbundle-${{ runner.os }}-${{ hashFiles('**/jbundle.toml') }}
    restore-keys: |
      jbundle-${{ runner.os }}-
```

## Reusing the CI JDK

`setup-java` sets `JAVA_HOME` automatically. jbundle detects it and skips the Adoptium download:

```yaml
- uses: actions/setup-java@v4
  with:
    distribution: temurin
    java-version: 21

# jbundle reuses JAVA_HOME — no extra download
- uses: avelino/jbundle@main
  with:
    input: .
    output: ./dist/myapp
```

## Using jbundle.toml

Instead of passing inputs, use a config file in your repo:

```toml
# jbundle.toml
java_version = 21
profile = "cli"
jvm_args = ["-Xmx512m"]
```

Then your workflow simplifies to:

```yaml
- uses: avelino/jbundle@main
  with:
    input: .
    output: ./dist/myapp
```

## Release Workflow

Create releases with binaries for all platforms:

```yaml
name: Release

on:
  push:
    tags: ["v*"]

jobs:
  build:
    strategy:
      matrix:
        include:
          - os: ubuntu-latest
            target: linux-x64
          - os: macos-14
            target: macos-aarch64

    runs-on: ${{ matrix.os }}

    steps:
      - uses: actions/checkout@v4

      - uses: actions/setup-java@v4
        with:
          distribution: temurin
          java-version: 21

      - uses: avelino/jbundle@main
        with:
          input: .
          output: ./myapp-${{ matrix.target }}
          target: ${{ matrix.target }}

      - uses: softprops/action-gh-release@v1
        with:
          files: ./myapp-${{ matrix.target }}
```

## Debug

Enable verbose logging:

```yaml
- uses: avelino/jbundle@main
  with:
    input: .
    output: ./dist/myapp
    args: "--verbose"
  env:
    RUST_LOG: debug
```

Or use dry-run to preview the build plan:

```yaml
- uses: avelino/jbundle@main
  with:
    input: .
    output: ./dist/myapp
    args: "--dry-run"
```

## Troubleshooting

### Build hangs at "Detecting build system"

Gradle downloading dependencies. Add caching:

```yaml
- uses: actions/cache@v4
  with:
    path: |
      ~/.gradle/caches
      ~/.gradle/wrapper
    key: gradle-${{ runner.os }}-${{ hashFiles('**/*.gradle*', '**/gradle-wrapper.properties') }}
```

### Out of memory

```yaml
- uses: avelino/jbundle@main
  with:
    input: .
    output: ./dist/myapp
    args: "--jvm-args '-Xmx2g'"
```
