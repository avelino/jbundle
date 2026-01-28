# Quick Start

Build your first self-contained binary in under 5 minutes.

## Basic Usage

```bash
# Build from project directory
jbundle build --input ./my-app --output ./dist/my-app

# Run the binary (no Java required)
./dist/my-app
```

## By Build System

### Clojure (deps.edn)

```bash
jbundle build --input ./my-clojure-app --output ./dist/app
```

Requires a `:build` alias with `tools.build` configured to produce an uberjar.

### Clojure (Leiningen)

```bash
jbundle build --input ./my-lein-app --output ./dist/app
```

Runs `lein uberjar` internally.

### Java (Maven)

```bash
jbundle build --input ./my-java-app --output ./dist/app
```

Runs `mvn package -DskipTests` internally. Requires a configured shade/assembly plugin for uberjar.

### Java (Gradle)

```bash
jbundle build --input ./my-gradle-app --output ./dist/app
```

Runs `gradle build -x test` internally. Requires a configured shadowJar/fatJar task.

### From Pre-built JAR

If you already have an uberjar:

```bash
jbundle build --input ./target/app.jar --output ./dist/app
```

This skips the build step and goes straight to packaging.

## What Happens

On first run of the output binary:

1. **Extract** — Runtime and app are extracted to `~/.jbundle/cache/`
2. **Generate AppCDS** — JVM creates shared class archive (one-time, ~2-5s)
3. **Run** — Application starts

On subsequent runs:

1. **Cache hit** — Everything already extracted
2. **Load AppCDS** — JVM loads pre-processed metadata
3. **Run** — Application starts fast (~200-350ms)

## Common Options

```bash
# Specify Java version
jbundle build --input . --output ./dist/app --java-version 21

# Use CLI profile (optimized for fast startup)
jbundle build --input . --output ./dist/app --profile cli

# Cross-compile for Linux
jbundle build --input . --output ./dist/app --target linux-x64

# Pass JVM arguments
jbundle build --input . --output ./dist/app --jvm-args "-Xmx512m"
```

## Next Steps

* [Configuration](../guide/configuration.md) — Use `jbundle.toml` to avoid repeating flags
* [JVM Profiles](../guide/profiles.md) — Understand `cli` vs `server` profiles
* [CLI Reference](../reference/cli.md) — Full command documentation
