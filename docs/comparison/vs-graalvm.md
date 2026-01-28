# jbundle vs GraalVM native-image

Both aim to simplify JVM application distribution. They take fundamentally different approaches.

## Quick Summary

| Tool | Approach | Trade-off |
|------|----------|-----------|
| **jbundle** | Bundle minimal JVM + JAR | Full compatibility, good startup |
| **GraalVM** | Compile to native code | Fast startup, limited compatibility |

## GraalVM native-image

GraalVM compiles JVM bytecode to native machine code ahead-of-time (AOT). The result is a true native executable — no JVM at runtime.

**Pros:**
* Instant startup (~10-50ms)
* Small binaries (~20-40MB)
* Lower memory footprint

**Cons:**
* Long build times (minutes)
* Compatibility issues (reflection, dynamic proxies)
* Complex configuration required
* Not everything works

## jbundle

jbundle bundles a minimal JVM runtime with your application. The JVM runs at runtime, but optimized for startup.

**Pros:**
* 100% JVM compatible
* Fast builds (seconds)
* No configuration required
* Everything that works on JVM works here

**Cons:**
* Larger binaries (~30-50MB)
* Slower startup than true native (~200-350ms)

## Detailed Comparison

| Aspect | jbundle | GraalVM native-image |
|--------|---------|---------------------|
| **Compatibility** | 100% JVM | Requires reflection config |
| **Build time** | Fast (jlink + packaging) | Slow (AOT compilation) |
| **Binary size** | ~30-50 MB | ~20-40 MB |
| **Startup (warm)** | ~200-350ms (AppCDS) | ~10-50ms |
| **Setup** | Just `jbundle` | GraalVM + native-image + config |
| **Debug** | Standard JVM tools | Limited |

## The Compatibility Problem

GraalVM uses closed-world analysis — it must know every class at compile time. This breaks:

* **Reflection** — Requires manual `reflect-config.json`
* **Dynamic proxies** — Requires `proxy-config.json`
* **Runtime class loading** — Not supported
* **Many libraries** — Especially older ones

You'll spend hours writing configuration files, debugging `ClassNotFoundException`, and discovering that library X doesn't support native-image.

### Example: Reflection Configuration

```json
// reflect-config.json
[
  {
    "name": "com.example.MyClass",
    "allDeclaredFields": true,
    "allDeclaredMethods": true,
    "allDeclaredConstructors": true
  },
  {
    "name": "com.fasterxml.jackson.databind.ObjectMapper",
    "allDeclaredMethods": true
  }
]
```

This must be maintained as your code changes. Miss one class? Runtime failure.

## jbundle's Approach

jbundle keeps the full JVM, optimizing startup through:

1. **Minimal runtime** — Only modules your app needs
2. **AppCDS** — Pre-parsed class metadata
3. **JVM profiles** — Tuned flags for CLI vs server
4. **CRaC** — Checkpoint/restore for near-native startup

Everything works. No configuration. No compatibility matrix.

## Performance Comparison

### Startup Time

| Scenario | jbundle (cli) | GraalVM |
|----------|---------------|---------|
| First run | ~800-1500ms | ~10-50ms |
| Subsequent runs | ~200-350ms | ~10-50ms |
| With CRaC | ~10-50ms | ~10-50ms |

### Build Time

| Project Size | jbundle | GraalVM |
|--------------|---------|---------|
| Small | ~5s | ~30s |
| Medium | ~10s | ~2-5min |
| Large | ~20s | ~10-20min |

## When to Use What

### Use GraalVM native-image when:

* Startup time is critical (serverless, CLI)
* Your dependencies are native-image compatible
* You have time to maintain reflection configs
* Binary size matters

### Use jbundle when:

* You need full JVM compatibility
* Build time matters (CI/CD)
* You don't want configuration overhead
* Libraries don't support native-image
* You want "it just works"

### Use both?

Some teams use:
* **jbundle** for development and testing (fast builds, full compat)
* **GraalVM** for production (optimal startup)

This works if your app is GraalVM-compatible.

## tl;dr

* **GraalVM** = true native compilation (when it works)
* **jbundle** = JVM with optimized startup (always works)

Choose based on your compatibility requirements and how much configuration you're willing to maintain.
