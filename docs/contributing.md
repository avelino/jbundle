# Contributing

Contributions are welcome!

## Getting Started

### Prerequisites

* [Rust toolchain](https://rustup.rs/) (1.70+)
* Git

### Setup

```bash
# Clone
git clone https://github.com/avelino/jbundle.git
cd jbundle

# Build
cargo build

# Run tests
cargo test

# Lint
cargo clippy -- -D warnings

# Format
cargo fmt
```

## Testing Changes

Run against example projects:

```bash
# Clojure (deps.edn)
cargo run -- build --input ./example/clojure-deps --output ./dist/app

# Clojure (Leiningen)
cargo run -- build --input ./example/clojure-lein --output ./dist/app

# Java (Maven)
cargo run -- build --input ./example/java-pom --output ./dist/app

# Java (Gradle)
cargo run -- build --input ./example/java-gradle --output ./dist/app

# Run the generated binary
./dist/app
```

With verbose logging:

```bash
RUST_LOG=debug cargo run -- build --input ./example/clojure-deps --output ./dist/app
```

## Pull Request Process

1. **Fork** the repository
2. **Create a branch** for your change (`git checkout -b feature/my-feature`)
3. **Make changes** with tests if applicable
4. **Run checks** (`cargo test && cargo clippy && cargo fmt --check`)
5. **Open a pull request**

### PR Guidelines

* Keep changes focused — one feature/fix per PR
* Include tests for new functionality
* Update documentation if behavior changes
* Follow existing code style

## Contribution Ideas

Looking for something to work on? Here are some areas that need help:

### Features

* Windows support
* Custom `jlink` module list override
* Compression options (zstd, xz)
* Homebrew formula
* Pre-built binary releases

### Documentation

* More language-specific examples (Kotlin, Scala)
* CI/CD integration guides
* Troubleshooting guide expansion

### Testing

* More integration tests
* Platform-specific test coverage
* Performance benchmarks

## Code Structure

```
src/
├── main.rs          # CLI and pipeline orchestration
├── config.rs        # BuildConfig, Target types
├── detect.rs        # Build system detection
├── build.rs         # JAR building
├── jlink.rs         # Runtime creation
├── error.rs         # PackError enum
├── jvm/
│   ├── adoptium.rs  # Adoptium API client
│   ├── download.rs  # HTTP download
│   └── cache.rs     # JDK extraction and caching
└── pack/
    ├── archive.rs   # tar.gz creation
    ├── stub.rs      # Shell stub generation
    └── mod.rs       # Final binary assembly
```

## Questions?

* Open an issue for bugs or feature requests
* Discussions welcome in pull requests

## License

By contributing, you agree that your contributions will be licensed under MIT.
