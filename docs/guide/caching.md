# Caching & Performance

jbundle uses a layered caching system to optimize both build time and runtime performance.

## How Caching Works

The output binary contains independent layers:

```
[stub script] [runtime.tar.gz] [app.jar.gz] [crac.tar.gz?]
```

Each layer is cached by content hash at `~/.jbundle/cache/`:

```
~/.jbundle/cache/
├── jdk-21-linux-x64/     # Downloaded JDK (reused across builds)
├── rt-abc123/            # Extracted runtime
├── app-def456/           # Extracted app + app.jsa
└── crac-ghi789/          # CRaC checkpoint (if enabled)
```

## Why This Matters

**Build time:** Changing only application code doesn't re-download the JDK or re-create the runtime.

**Run time:** Updating your app doesn't re-extract the runtime layer. Only the app layer is replaced.

**CI/CD:** Multiple builds with the same JDK version share the cached download.

## Startup Performance

### First Run vs Subsequent Runs

| Metric | First Run | Subsequent Runs |
|--------|-----------|-----------------|
| **What happens** | Extract layers + generate AppCDS | Load from cache |
| **Overhead** | +2-5s | None |
| **Startup (cli)** | ~800-1500ms | ~200-350ms |
| **Startup (server)** | ~1000-2000ms | ~400-600ms |

### Why First Run is Slower

1. **Extraction** — Compressed layers are decompressed to cache
2. **AppCDS generation** — JVM creates `.jsa` file with pre-processed classes

This is a one-time cost per app version.

### Why Subsequent Runs are Faster

1. **Cache hit** — Everything already extracted
2. **AppCDS loaded** — JVM skips parsing and verification
3. **Profile flags** — Optimized JVM configuration

## AppCDS (Class Data Sharing)

Enabled by default on JDK 19+. The JVM automatically generates a shared archive on first run:

```
~/.jbundle/cache/app-<hash>/app.jsa
```

This file contains:
* Pre-parsed class metadata
* Pre-verified bytecode
* Pre-computed class layouts

Result: 60-75% faster startup on subsequent runs.

### Disabling AppCDS

```bash
jbundle build --input . --output ./app --no-appcds
```

Useful if you observe issues with specific libraries.

## CRaC (Coordinated Restore at Checkpoint)

Optional feature for near-instant startup (~10-50ms).

### How It Works

1. Application starts and warms up
2. Checkpoint is created (memory snapshot)
3. Checkpoint is bundled in the binary
4. Subsequent runs restore from checkpoint

### Requirements

* Linux only
* JDK with CRaC support (e.g., Azul Zulu with CRaC)

### Usage

```bash
jbundle build --input . --output ./app --crac
```

Falls back to AppCDS if restore fails.

## Cache Management

### View Cache Info

```bash
jbundle info
```

Shows cached JDKs, runtimes, and apps.

### Clean Cache

```bash
jbundle clean
```

Removes all cached data.

## Performance Tips

1. **Use `--profile cli`** for command-line tools
2. **Keep AppCDS enabled** (default) for best startup
3. **Consider CRaC** for Linux deployments where startup is critical
4. **Pre-warm in CI** by running the binary once to generate AppCDS
