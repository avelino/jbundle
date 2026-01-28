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
```

The JDK is downloaded for the target platform, not the host.

## Platform Detection

When no `--target` is specified, jbundle detects the current platform:

| Host OS | Host Arch | Default Target |
|---------|-----------|----------------|
| macOS | ARM64 | `macos-aarch64` |
| macOS | x86_64 | `macos-x64` |
| Linux | x86_64 | `linux-x64` |
| Linux | ARM64 | `linux-aarch64` |

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

Windows is not currently supported. Planned for a future release.

## Notes

* **CRaC** is Linux-only (checkpoint/restore requires Linux kernel features)
* **Binary format** differs between platforms (ELF on Linux, Mach-O on macOS)
* **Shell stub** uses `/bin/sh` which is available on all Unix-like systems
