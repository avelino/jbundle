# Supported Platforms

jbundle can create binaries for multiple platforms.

## Available Targets

| Target | Architecture | OS |
|--------|--------------|-----|
| `linux-x64` | x86_64 | Linux |
| `linux-aarch64` | ARM64 | Linux |
| `macos-x64` | x86_64 | macOS |
| `macos-aarch64` | ARM64 (Apple Silicon) | macOS |

## Usage

```bash
# Build for current platform (default)
jbundle build --input . --output ./app

# Build for Linux x64
jbundle build --input . --output ./app --target linux-x64

# Build for Linux ARM64
jbundle build --input . --output ./app --target linux-aarch64

# Build for macOS Intel
jbundle build --input . --output ./app --target macos-x64

# Build for macOS Apple Silicon
jbundle build --input . --output ./app --target macos-aarch64
```

Or in `jbundle.toml`:

```toml
target = "linux-x64"
```

## Cross-Compilation

jbundle supports cross-compilation. You can build Linux binaries from macOS:

```bash
# On macOS, build for Linux
jbundle build --input . --output ./app-linux --target linux-x64

# Preview what cross-compilation will do
jbundle build --input . --output ./app-linux --target linux-x64 --dry-run
```

When cross-compiling, jbundle downloads **two** JDKs:

1. **Host JDK** — Used to run `jdeps` (module detection) and `jlink` (runtime creation)
2. **Target JDK** — Provides the `jmods` directory for the target platform

This ensures that `jdeps` and `jlink` can execute on your machine while producing a runtime for the target platform.

## Platform Detection

When no `--target` is specified, jbundle detects the current platform:

| Host OS | Host Arch | Default Target |
|---------|-----------|----------------|
| macOS | ARM64 | `macos-aarch64` |
| macOS | x86_64 | `macos-x64` |
| Linux | x86_64 | `linux-x64` |
| Linux | ARM64 | `linux-aarch64` |
| Windows | x86_64 | `windows-x64` |

## CI/CD Example

Build for multiple platforms in GitHub Actions:

```yaml
jobs:
  build:
    strategy:
      matrix:
        target: [linux-x64, linux-aarch64, macos-x64, macos-aarch64]
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Build
        run: jbundle build --input . --output ./dist/app-${{ matrix.target }} --target ${{ matrix.target }}
      - uses: actions/upload-artifact@v4
        with:
          name: app-${{ matrix.target }}
          path: ./dist/app-${{ matrix.target }}
```

## Windows Support

> **Help wanted:** Windows support is maintained on a best-effort basis. If you use jbundle on Windows and want to help keep it working, we'd love your contributions — see [open issues](https://github.com/avelino/jbundle/issues).

jbundle runs on Windows as a **build host** — pre-compiled binaries are available for Windows x86_64. You can use jbundle on Windows to build Linux and macOS binaries via cross-compilation.

**What works on Windows:**

* Installing and running jbundle (`jbundle.exe`)
* Building uberjars from JVM projects
* Cross-compiling to Linux/macOS targets
* Using `--java-home` or `JAVA_HOME` to reuse a local JDK

**Limitation:** The output binary uses a Unix shell stub (`/bin/sh`), so Windows is not supported as an **output target**. The binaries jbundle produces run on Linux and macOS.

### Install on Windows

```powershell
irm https://raw.githubusercontent.com/avelino/jbundle/main/install.ps1 | iex
```

### Example: Build Linux binary from Windows

```powershell
jbundle build --input . --output ./dist/myapp --target linux-x64
```

## Notes

* **CRaC** is Linux-only (checkpoint/restore requires Linux kernel features)
* **Binary format** differs between platforms (ELF on Linux, Mach-O on macOS)
* **Shell stub** uses `/bin/sh` which is available on all Unix-like systems
* **Windows** is supported as a build host but not as an output target
