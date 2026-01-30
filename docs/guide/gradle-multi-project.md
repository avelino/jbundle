# Gradle Multi-Project

jbundle automatically detects and handles Gradle multi-project builds with multiple application subprojects.

## How It Works

When jbundle detects `settings.gradle.kts` (or `.gradle`), it:

1. Parses included subprojects
2. Scans each for the `application` plugin
3. Extracts `mainClass`, `mainModule`, and `addModules` configuration
4. Offers selection or builds all with `--all`

## Detection

jbundle looks for these patterns in `build.gradle.kts`:

```kotlin
plugins {
    id("application")
}

application {
    mainClass.set("com.example.Main")
}
```

And module configuration:

```kotlin
javaModulePackaging {
    addModules.add("jdk.incubator.vector")
}
```

## Single Subproject

### Interactive Selection

When multiple application subprojects exist:

```bash
jbundle build --output ./dist/app
```

```
Multiple application subprojects found:
  [1] app - com.example.App
  [2] cli - com.example.Cli
  [3] server - com.example.Server

Tip: Add 'gradle_project = "app"' to jbundle.toml to skip this prompt

Select subproject [1-3]:
```

### CLI Flag

Skip the prompt with `--gradle-project`:

```bash
jbundle build --output ./dist/app --gradle-project app
```

### Configuration File

Set default in `jbundle.toml`:

```toml
gradle_project = "app"
```

## All Subprojects

Build every application subproject with `--all`:

```bash
jbundle build --output ./dist --all
```

Each binary is placed in `{output}/{subproject-name}`:

```
./dist/
├── app
├── cli
└── server
```

## Module Handling

### Automatic Detection

jbundle combines:

1. **jdeps analysis** — Scans JAR for required modules
2. **Gradle config** — Extracts `addModules.add(...)` from build files

Both are merged and deduplicated for the jlink runtime.

### Manual Override

Bypass automatic detection with `--modules`:

```bash
jbundle build --output ./dist/app \
  --gradle-project app \
  --modules java.base,java.sql,jdk.incubator.vector
```

Or in configuration:

```toml
# jbundle.toml
gradle_project = "app"
modules = ["java.base", "java.sql", "jdk.incubator.vector"]
```

## Reusing Existing Runtime

If Gradle already built a jlink image, skip the jlink step:

```bash
jbundle build --output ./dist/app \
  --gradle-project app \
  --jlink-runtime ./app/build/jlink
```

Common locations jbundle checks:
- `{subproject}/build/jlink/`
- `{subproject}/build/image/`
- `{subproject}/build/jpackage/images/app-image/`

## Build Process

For subprojects, jbundle runs:

1. `:{subproject}:shadowJar` (preferred, produces fat JAR)
2. Falls back to `:{subproject}:build` if shadowJar unavailable

Then looks for JAR in:
- `{subproject}/build/libs/*-all.jar`
- `{subproject}/build/libs/*-uber.jar`
- `{subproject}/build/libs/*.jar`

## Configuration Reference

New options for multi-project builds:

### CLI Flags

| Flag | Description |
|------|-------------|
| `--gradle-project <NAME>` | Build specific subproject |
| `--all` | Build all application subprojects |
| `--modules <LIST>` | Manual module list (comma-separated) |
| `--jlink-runtime <PATH>` | Reuse existing jlink runtime |

### jbundle.toml

```toml
# Subproject to build by default
gradle_project = "app"

# Manual module override
modules = ["java.base", "java.sql"]

# Reuse existing runtime
jlink_runtime = "./build/jlink"
```

## Examples

### Simple Multi-Project

```bash
# Interactive selection
jbundle build --output ./dist/app

# Specific subproject
jbundle build --output ./dist/cli --gradle-project cli

# All at once
jbundle build --output ./dist --all
```

### With Custom Modules

```bash
jbundle build \
  --output ./dist/app \
  --gradle-project app \
  --modules java.base,java.desktop,java.sql
```

### Reusing Gradle's jlink

```bash
# First, run Gradle's jlink task
./gradlew :app:jlink

# Then use it with jbundle
jbundle build \
  --output ./dist/app \
  --gradle-project app \
  --jlink-runtime ./app/build/image
```

## Troubleshooting

### "No application subproject found"

Check that subprojects have the `application` plugin applied:

```kotlin
plugins {
    id("application")
}
```

### "Subproject 'x' not found"

Verify the subproject name matches what's in `settings.gradle.kts`:

```kotlin
include("app")  // Use "app", not ":app"
```

### Module errors at runtime

If the app fails with `java.lang.module` errors, modules are missing. Use `--modules` to add them:

```bash
jbundle build --modules java.base,java.sql,java.logging,...
```

### shadowJar not available

If your project doesn't use Shadow plugin, ensure the regular build produces a fat JAR with all dependencies bundled.
