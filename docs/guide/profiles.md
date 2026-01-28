# JVM Profiles

The `--profile` flag selects a set of optimized JVM flags for your workload.

## Available Profiles

### server (default)

Standard HotSpot behavior. No additional flags.

**Best for:**
* Long-running services
* Web servers
* Applications where throughput matters more than startup

**Characteristics:**
* Full tiered compilation (C1 → C2)
* G1GC (default garbage collector)
* Higher memory overhead
* Peak performance after warmup

### cli

Optimized for short-lived processes and CLI tools.

**Best for:**
* Command-line tools
* Scripts
* One-shot utilities
* Serverless functions

**Characteristics:**
* Tiered compilation with C1 only (skip C2)
* SerialGC (simpler, faster startup)
* Reduced code cache
* ~200-350ms startup (with AppCDS)

## Usage

```bash
# CLI profile
jbundle build --input . --output ./dist/app --profile cli

# Server profile (default)
jbundle build --input . --output ./dist/app --profile server
```

Or in `jbundle.toml`:

```toml
profile = "cli"
```

## Performance Comparison

| Metric | cli | server |
|--------|-----|--------|
| Startup (cold) | ~800-1500ms | ~1000-2000ms |
| Startup (warm) | ~200-350ms | ~400-600ms |
| Peak throughput | Lower | Higher |
| Memory footprint | Lower | Higher |
| Warmup time | Faster | Slower |

## When to Use Each

```
cli profile:
  ✓ Run once and exit
  ✓ Interactive commands
  ✓ Scripts and automation
  ✓ Startup time is critical

server profile:
  ✓ Long-running processes
  ✓ Web servers
  ✓ Background services
  ✓ Throughput is critical
```

## Combining with AppCDS and CRaC

Profiles work alongside other optimizations:

```bash
# CLI + AppCDS (default) → ~200-350ms
jbundle build --input . --output ./app --profile cli

# CLI + CRaC → ~10-50ms (Linux only)
jbundle build --input . --output ./app --profile cli --crac

# Server + no AppCDS → standard JVM startup
jbundle build --input . --output ./app --profile server --no-appcds
```
