# JDK Versions

jbundle downloads JDK runtimes from [Adoptium](https://adoptium.net/).

## Supported Versions

| Version | Type | Status | Notes |
|---------|------|--------|-------|
| `11` | LTS | Supported | |
| `17` | LTS | Supported | |
| `21` | LTS | **Default** | Recommended |
| `22` | STS | Supported | |
| `23` | STS | Supported | |
| `24` | STS | Supported | |
| `25` | LTS | Supported | |

**LTS** = Long Term Support (recommended for production)
**STS** = Short Term Support

## Usage

```bash
# Use default (21)
jbundle build --input . --output ./app

# Specify version
jbundle build --input . --output ./app --java-version 17
jbundle build --input . --output ./app --java-version 25
```

Or in `jbundle.toml`:

```toml
java_version = 21
```

## Why Not Java 8?

jbundle requires `jlink` and `jdeps`, which were introduced in Java 9. These tools are essential for:

* **jdeps** — Analyzing module dependencies
* **jlink** — Creating minimal custom runtimes

Java 8 predates the module system (JPMS) and doesn't include these tools.

## Recommendations

### Production

Use **LTS versions** (11, 17, 21, 25):
* Longer support lifecycle
* More stable
* Security updates for years

### New Projects

Use **Java 21** (current LTS):
* Modern features (virtual threads, pattern matching)
* Long-term support until 2029+
* Best balance of features and stability

### Specific Features

| If you need... | Minimum version |
|----------------|-----------------|
| Virtual threads | 21 |
| Pattern matching for switch | 21 |
| Records | 16 |
| Text blocks | 15 |
| `var` keyword | 10 |

## AppCDS Compatibility

AppCDS (automatic shared archive) requires JDK 19+. On older JDKs:

* JDK 11-18: Manual CDS configuration (not automatic)
* JDK 19+: Automatic AppCDS via `-XX:+AutoCreateSharedArchive`

For best startup performance, use JDK 21 or newer.

## JDK Download & Cache

JDKs are downloaded from the Adoptium API and cached locally:

```
~/.jbundle/cache/jdk-21-linux-x64/
~/.jbundle/cache/jdk-21-macos-aarch64/
```

Downloads are verified with SHA256 checksums. Re-running builds with the same JDK version reuses the cached download.

## Adoptium vs Other JDKs

jbundle uses [Eclipse Temurin](https://adoptium.net/temurin/) (Adoptium's distribution) because:

* Open source
* Free for commercial use
* Reliable API for automated downloads
* Available for all supported platforms
* TCK certified (passes Java compatibility tests)

Custom JDK distributions (Oracle, Azul, Amazon Corretto) are not currently supported.

### Exception: CRaC

For CRaC support (`--crac`), you need a JDK with CRaC patches. Currently, this means [Azul Zulu with CRaC](https://www.azul.com/products/components/crac/). CRaC support in Adoptium/Temurin is planned but not yet available.
