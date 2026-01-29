# Example: JabRef

[JabRef](https://github.com/JabRef/jabref) is a complex Gradle multi-project with multiple application subprojects. This guide shows how to package it with jbundle.

## Project Structure

JabRef uses a multi-module Gradle setup:

```
jabref/
├── settings.gradle.kts
├── build.gradle.kts
├── jabkit/
│   └── build.gradle.kts      # CLI tool (application plugin)
├── jabgui/
│   └── build.gradle.kts      # GUI application (application plugin)
├── jabsrv-cli/
│   └── build.gradle.kts      # Server CLI (application plugin)
├── jabls-cli/
│   └── build.gradle.kts      # Language server (application plugin)
└── jablib/
    └── build.gradle.kts      # Shared library (no application)
```

## Auto-Detection

jbundle automatically detects multi-project builds by parsing `settings.gradle.kts` and scanning subprojects for the `application` plugin.

```bash
cd jabref
jbundle build --output ./dist/app
```

When multiple application subprojects are found, jbundle prompts for selection:

```
Multiple application subprojects found:
  [1] jabkit - org.jabref.cli.JabKit
  [2] jabgui - org.jabref.gui.JabRefGUI
  [3] jabsrv-cli - org.jabref.http.server.Server
  [4] jabls-cli - org.jabref.language.JabLS

Tip: Add 'gradle_project = "jabkit"' to jbundle.toml to skip this prompt

Select subproject [1-4]:
```

## Building a Single Subproject

### Using CLI Flag

```bash
jbundle build --input . --output ./dist/jabkit --gradle-project jabkit
```

### Using Configuration File

```toml
# jbundle.toml
gradle_project = "jabkit"
java_version = 21
profile = "cli"
```

Then simply:

```bash
jbundle build --output ./dist/jabkit
```

## Building All Subprojects

Use `--all` to build every application subproject at once:

```bash
jbundle build --input . --output ./dist --all
```

Output:

```
Building 4 application subprojects:
  - jabkit (org.jabref.cli.JabKit)
  - jabgui (org.jabref.gui.JabRefGUI)
  - jabsrv-cli (org.jabref.http.server.Server)
  - jabls-cli (org.jabref.language.JabLS)

━━━ Building jabkit ━━━
[1/6] Detecting build system.............. Gradle multi-project (jabkit)
[2/6] Building uberjar.................... jabkit-all.jar
[3/6] Downloading JDK 21.................. ready
[4/6] Analyzing module dependencies....... 42 modules
[5/6] Creating minimal runtime............ done
[6/6] Packing binary...................... ./dist/jabkit (48.2 MB)

━━━ Building jabgui ━━━
...

━━━ Build complete ━━━
Built 4 binaries:
  - ./dist/jabkit
  - ./dist/jabgui
  - ./dist/jabsrv-cli
  - ./dist/jabls-cli
```

## Module Detection

JabRef uses Java modules with incubator features. jbundle extracts `addModules` from `build.gradle.kts`:

```kotlin
// jabkit/build.gradle.kts
javaModulePackaging {
    addModules.add("jdk.incubator.vector")
}
```

These modules are automatically included in the jlink runtime, combined with jdeps analysis.

### Manual Module Override

If auto-detection misses modules or you want full control:

```bash
jbundle build --output ./dist/jabkit \
  --gradle-project jabkit \
  --modules java.base,java.sql,java.xml,jdk.incubator.vector
```

Or in configuration:

```toml
# jbundle.toml
gradle_project = "jabkit"
modules = ["java.base", "java.sql", "java.xml", "jdk.incubator.vector"]
```

## Reusing Existing jlink Runtime

If JabRef's Gradle build already created a jlink image, reuse it:

```bash
jbundle build --output ./dist/jabkit \
  --gradle-project jabkit \
  --jlink-runtime ./jabkit/build/jlink
```

This skips the JDK download and jlink steps, using the pre-built runtime.

## Complete Configuration

Full `jbundle.toml` for JabRef:

```toml
# jbundle.toml

# Build jabkit by default
gradle_project = "jabkit"

# Use Java 21 (required by JabRef)
java_version = 21

# CLI profile for fast startup
profile = "cli"

# JVM settings for JabRef
jvm_args = ["-Xmx1g", "--enable-preview"]

# Shrink the JAR
shrink = true
```

## CI/CD Integration

### GitHub Actions

```yaml
name: Build Binaries

on:
  release:
    types: [created]

jobs:
  build:
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest]
        include:
          - os: ubuntu-latest
            target: linux-x64
          - os: macos-latest
            target: macos-aarch64

    runs-on: ${{ matrix.os }}

    steps:
      - uses: actions/checkout@v4

      - name: Install jbundle
        run: cargo install jbundle

      - name: Build all applications
        run: |
          jbundle build \
            --input . \
            --output ./dist \
            --target ${{ matrix.target }} \
            --all

      - name: Upload artifacts
        uses: actions/upload-artifact@v4
        with:
          name: binaries-${{ matrix.target }}
          path: ./dist/*
```

## Troubleshooting

### "No application subproject found"

Ensure subprojects have the `application` plugin:

```kotlin
plugins {
    id("application")
}

application {
    mainClass.set("com.example.Main")
}
```

### "shadowJar task not found"

jbundle tries `:subproject:shadowJar` first, then falls back to `:subproject:build`. If you're not using Shadow plugin, ensure regular build produces a fat JAR.

### Module resolution errors

If jdeps fails to detect all required modules, use `--modules` to specify them manually:

```bash
jbundle build --modules java.base,java.desktop,java.sql,...
```

### Build takes too long

For development iteration, consider:

1. Using `--jlink-runtime` with a pre-built runtime
2. Building only the subproject you're working on (not `--all`)
