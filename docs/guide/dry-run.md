# Dry-Run Mode

Preview what jbundle will do without executing anything.

## Usage

```bash
jbundle build --input ./my-app --output ./dist/app --dry-run
```

## What It Shows

```
[dry-run] Build plan for ./my-app → ./dist/app

  Build system:  DepsEdn (clojure -T:build uber)
  Target:        Target { os: Linux, arch: X86_64 }
  JDK version:   21
  JDK source:    cached ✓
  Modules:       auto-detect (jdeps)
  Runtime:       create via jlink
  Profile:       cli
  Shrink:        enabled
  AppCDS:        enabled
  CRaC:          disabled
  JVM args:      -Xmx512m

  No actions taken.
```

| Field | Description |
|-------|-------------|
| **Build system** | Detected build tool and command that would run |
| **Target** | OS and architecture of the output binary |
| **Cross-compile** | Shown when target differs from host platform |
| **JDK version** | Which JDK version will be used |
| **JDK source** | Whether the JDK is cached, local (`--java-home`), or needs download |
| **Modules** | Auto-detect via jdeps or manual override |
| **Runtime** | New via jlink or reusing existing (`--jlink-runtime`) |
| **Profile** | `cli` (fast startup) or `server` (throughput) |
| **Shrink/AppCDS/CRaC** | Feature toggles |
| **JVM args / Build args** | Extra arguments, if any |

## When to Use

**Validating config before a long build:**

```bash
# Check that jbundle.toml is picking up the right settings
jbundle build --input . --output ./app --dry-run
```

**Verifying cross-compilation setup:**

```bash
# Confirm host + target JDK strategy
jbundle build --input . --output ./app --target linux-x64 --dry-run
```

**Debugging CI:**

```bash
# Add to your CI pipeline before the real build
jbundle build --input . --output ./dist/app --dry-run
jbundle build --input . --output ./dist/app
```

**Checking build system detection:**

```bash
# Repos with multiple build files (deps.edn + project.clj)
jbundle build --input ./my-project --output ./app --dry-run
```

## Configuration

Dry-run is CLI-only — there's no `jbundle.toml` equivalent. All other config options (`--java-version`, `--profile`, `--target`, etc.) work normally with `--dry-run` to preview their effect.
