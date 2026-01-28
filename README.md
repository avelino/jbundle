# jbundle

Package JVM applications into self-contained binaries. No JVM installation required.

Transforms JVM applications (Clojure, Java, Kotlin, Scala, Groovy) into self-contained binaries. Previously known as `clj-pack`, renamed to reflect support for all JVM languages.

## Why jbundle?

GraalVM native-image has slow compilations, complex reflection configuration, and library incompatibilities. jbundle bundles a minimal JVM runtime with your uberjar into a single executable—full JVM compatibility, no external dependencies.

## Quick Start

    # Install
    git clone https://github.com/avelino/jbundle.git
    cd jbundle
    cargo install --path .

    # Build your app
    jbundle build --input ./my-app --output ./dist/my-app

    # Run (no Java required)
    ./dist/my-app

## Documentation

[Full documentation](https://jbundle.avelino.run) available

## License

MIT
