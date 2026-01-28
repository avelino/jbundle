# Configuration

Avoid repeating flags by creating a `jbundle.toml` in your project root.

## Configuration File

```toml
# jbundle.toml

java_version = 21
target = "linux-x64"
jvm_args = ["-Xmx512m", "-XX:+UseZGC"]
profile = "cli"
appcds = true
crac = false
```

All fields are optional.

## Options

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `java_version` | integer | `21` | JDK version to bundle |
| `target` | string | current platform | Target platform (`linux-x64`, `macos-aarch64`, etc.) |
| `jvm_args` | array | `[]` | JVM arguments passed at runtime |
| `profile` | string | `"server"` | JVM profile (`"cli"` or `"server"`) |
| `appcds` | boolean | `true` | Enable AppCDS for faster startup |
| `crac` | boolean | `false` | Enable CRaC checkpoint (Linux only) |

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

Standard server configuration:

```toml
# jbundle.toml
java_version = 21
profile = "server"
jvm_args = ["-Xmx1g", "-XX:+UseZGC"]
```

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

## Environment Variables

For debugging, set `RUST_LOG` to control jbundle's logging:

```bash
# Show debug output
RUST_LOG=debug jbundle build --input . --output ./dist/app

# Show only warnings
RUST_LOG=warn jbundle build --input . --output ./dist/app
```
