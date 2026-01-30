# Configuration

Avoid repeating flags by creating a `jbundle.toml` in your project root.

## Configuration File

```toml
# jbundle.toml

java_version = 21
target = "linux-x64"
jvm_args = ["-Xmx512m", "-XX:+UseZGC"]
profile = "cli"
shrink = true
appcds = true
crac = false

# Gradle multi-project options
gradle_project = "app"
modules = ["java.base", "java.sql"]
jlink_runtime = "./build/jlink"
```

All fields are optional.

## Options

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `java_version` | integer | `21` | JDK version to bundle |
| `target` | string | current platform | Target platform (`linux-x64`, `macos-aarch64`, etc.) |
| `jvm_args` | array | `[]` | JVM arguments passed at runtime |
| `profile` | string | `"server"` | JVM profile (`"cli"` or `"server"`) |
| `shrink` | boolean | `false` | Shrink uberjar by removing non-essential files |
| `appcds` | boolean | `true` | Enable AppCDS for faster startup |
| `crac` | boolean | `false` | Enable CRaC checkpoint (Linux only) |
| `gradle_project` | string | — | Gradle subproject to build (for multi-project) |
| `modules` | array | — | Manual module list (bypasses jdeps detection) |
| `jlink_runtime` | string | — | Path to existing jlink runtime to reuse |

## Precedence

Configuration values are resolved in this order (highest to lowest):

1. **CLI flags** — `--java-version 17` overrides everything
2. **jbundle.toml** — Project-level defaults
3. **Internal defaults** — Built-in values

## Examples

### CLI Tool

Optimized for fast startup:

```toml
# jbundle.toml
java_version = 21
profile = "cli"
jvm_args = ["-Xmx256m"]
```

### Microservice

Standard server configuration with custom GC:

```toml
# jbundle.toml
java_version = 21
profile = "server"  # Important: use "server" when specifying custom GC
jvm_args = ["-Xmx1g", "-XX:+UseZGC"]
```

> **Note:** When using a custom garbage collector like ZGC, always use `profile = "server"`. The `"cli"` profile includes `-XX:+UseSerialGC`, and the JVM cannot use multiple GCs simultaneously. jbundle will detect this conflict and fail with a helpful error message.

### Cross-Platform Build

Targeting Linux from macOS:

```toml
# jbundle.toml
java_version = 21
target = "linux-x64"
```

### Maximum Performance (Linux)

With CRaC for instant startup:

```toml
# jbundle.toml
java_version = 21
profile = "cli"
crac = true
```

### Gradle Multi-Project

For complex projects like JabRef:

```toml
# jbundle.toml
gradle_project = "jabkit"
java_version = 21
profile = "cli"
jvm_args = ["-Xmx1g"]
```

### With Custom Modules

When jdeps detection is insufficient:

```toml
# jbundle.toml
modules = ["java.base", "java.sql", "java.desktop", "jdk.incubator.vector"]
```

### Reusing Existing Runtime

Skip jlink if you have a pre-built runtime:

```toml
# jbundle.toml
jlink_runtime = "./build/jlink"
```

## Environment Variables

For debugging, set `RUST_LOG` to control jbundle's logging:

```bash
# Show debug output
RUST_LOG=debug jbundle build --input . --output ./dist/app

# Show only warnings
RUST_LOG=warn jbundle build --input . --output ./dist/app
```
