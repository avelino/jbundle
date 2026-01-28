# jbundle

Package JVM applications into self-contained binaries. No JVM installation required to run the output.

## What is jbundle?

**jbundle** transforms JVM applications (Clojure, Java, Kotlin, Scala, Groovy) into self-contained binaries. Think of it as the Go/Rust distribution experience for the JVM world.

```bash
# Build your app
jbundle build --input ./my-app --output ./dist/my-app

# Run anywhere (no Java required)
./dist/my-app
```

## Why jbundle?

The conventional deployment approach requires both a JAR and a JVM on the target machine. GraalVM native-image is an alternative, but presents challenges: slow compilations, complex reflection configuration, and library incompatibilities.

**jbundle offers a practical solution:** bundle a minimal JVM runtime with your uberjar into a single executable. The result is a single file, no external dependencies, with full JVM compatibility.

## Key Features

| Feature | Description |
|---------|-------------|
| **Single binary** | One file to distribute, like Go or Rust |
| **No JVM required** | Runtime is bundled inside the binary |
| **Fast startup** | AppCDS + profiles achieve ~200-350ms startup |
| **Full compatibility** | Everything that works on JVM works here |
| **Multiple languages** | Clojure, Java, Kotlin, Scala, Groovy |
| **Smart caching** | Layers cached independently by content hash |

## How It Works

```
detect → build uberjar → download JDK → jdeps → jlink → pack
```

1. **Detect** — Identifies your build system (deps.edn, project.clj, pom.xml, build.gradle)
2. **Build** — Runs the appropriate build command to create an uberjar
3. **Download JDK** — Fetches JDK from Adoptium (cached locally)
4. **Analyze** — Uses jdeps to detect required modules
5. **Minimize** — Creates minimal runtime with jlink (~30-50 MB)
6. **Package** — Bundles everything into a single executable

## Quick Comparison

| Aspect | jbundle | GraalVM native-image |
|--------|---------|---------------------|
| **Compatibility** | 100% JVM compatible | Requires reflection config |
| **Build time** | Fast | Slow (AOT compilation) |
| **Startup** | ~200-350ms (AppCDS) | ~10-50ms |
| **Setup** | Just `jbundle` | GraalVM + native-image + config |

## Next Steps

* [Installation](getting-started/installation.md) — Get jbundle on your system
* [Quick Start](getting-started/quick-start.md) — Build your first binary
* [Configuration](guide/configuration.md) — Customize with `jbundle.toml`
