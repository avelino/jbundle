# jbundle vs jpackage

They solve different problems.

## Quick Summary

| Tool | Purpose | Output |
|------|---------|--------|
| **jbundle** | Native-feeling binaries | Single executable |
| **jpackage** | Traditional installers | .exe, .msi, .dmg, .deb, .rpm |

## jpackage: Distribution via Installers

**jpackage** is included in JDK 14+. It creates platform-specific installers:

* Windows: `.exe`, `.msi`
* macOS: `.dmg`, `.pkg`
* Linux: `.deb`, `.rpm`

The workflow is traditional:

```
User downloads installer → Runs installer → App installed to system → Launch from menu
```

jpackage bundles a full JVM runtime, but that's where optimization ends. No startup improvements, no runtime tuning. It's essentially "take your JAR, wrap it with a JVM, generate an installer."

**Best for:** Desktop applications where users expect traditional installation.

## jbundle: Native-Feeling Binaries

**jbundle** creates a single executable binary. The workflow mirrors Go or Rust:

```
Download → chmod +x → Run
```

No installer. No installation step. No system directories. The binary is self-contained — move it anywhere, copy to a server, distribute via curl.

**Best for:** CLI tools, microservices, serverless functions.

## Detailed Comparison

| Aspect | jbundle | jpackage |
|--------|---------|----------|
| **Output** | Single executable | Platform installers |
| **User experience** | Download → run | Download → install → run |
| **JVM size** | Minimal (~30-50MB via jlink) | Full JDK (~300MB) |
| **Startup optimization** | AppCDS, CRaC, profiles | None |
| **Distribution** | curl, cp, scp | App stores, package managers |
| **System integration** | None | File associations, shortcuts |
| **Target audience** | Developers, DevOps | End users |

## Why Startup Time Matters

The JVM is notorious for slow startup. A simple "hello world" can take 500ms+. For CLI tools or short-lived processes, this is unacceptable.

jbundle attacks this from multiple angles:

### 1. Minimal Runtime (jlink)

Instead of the full JDK (~300MB), jbundle uses `jdeps` to detect which modules your app uses, then `jlink` to create a minimal runtime (~30-50MB). Less code to load = faster startup.

### 2. AppCDS (Class Data Sharing)

On first run, the JVM generates a shared archive with pre-parsed class metadata. Subsequent runs load this cache directly, cutting startup by 60-75%.

### 3. Profile-Specific JVM Flags

The `--profile cli` option configures the JVM for short-lived processes:
* C1-only compilation (skip C2)
* SerialGC (simpler, faster startup)
* Reduced code cache

Result: ~200-350ms startup for CLI tools.

### 4. CRaC (Coordinated Restore at Checkpoint)

On supported JDKs, jbundle can create a checkpoint of your warmed-up app. Subsequent runs restore in 10-50ms — essentially native binary territory.

## When to Use What

### Use jpackage when:

* Building desktop applications with GUIs
* Users expect traditional installation
* You need system integration (file associations, shortcuts)
* Distributing through app stores

### Use jbundle when:

* Building CLI tools
* Building microservices or serverless functions
* You want Go/Rust-style distribution
* Startup time matters
* Deploying to servers or containers

## tl;dr

* **jpackage** = installers for desktop apps
* **jbundle** = native-feeling binaries with optimized startup
