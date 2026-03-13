# jbundle

Package JVM applications into self-contained binaries. No JVM installation required.

Transforms JVM applications (Clojure, Java, Kotlin, Scala, Groovy) into self-contained binaries. Previously known as `clj-pack`, renamed to reflect support for all JVM languages.

## Why jbundle?

GraalVM native-image has slow compilations, complex reflection configuration, and library incompatibilities. jbundle bundles a minimal JVM runtime with your uberjar into a single executable—full JVM compatibility, no external dependencies.

## Quick Start

```bash
# Install (macOS/Linux)
curl -sSL https://raw.githubusercontent.com/avelino/jbundle/main/install.sh | sh

# Or via Homebrew
brew tap avelino/jbundle
brew install jbundle

# Build your app
jbundle build --input ./my-app --output ./dist/my-app

# Run (no Java required)
./dist/my-app

# Preview build plan without executing
jbundle build --input ./my-app --output ./dist/my-app --dry-run
```

## Documentation

[Full documentation](https://jbundle.avelino.run) available

## License

MIT
