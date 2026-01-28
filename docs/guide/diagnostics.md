# Error Diagnostics

When a build fails, jbundle displays structured diagnostics with source context.

## Diagnostic Format

jbundle parses build errors and presents them in a familiar format, similar to `rustc`:

```
error: Unable to resolve symbol: prntln
 --> src/example/core.clj:9:5
   |
 7 | (defn process-data [data]
 8 |   (let [result (map inc data)]
 9 |     (prntln "Processing:" result)
   |     ^^^^^^^ symbol not found
10 |     (reduce + result)))
```

## What You Get

* **Error type** — What went wrong
* **Location** — File, line, and column
* **Source context** — Surrounding code with the error highlighted
* **Explanation** — When available, what the error means

## Supported Build Systems

Diagnostics work with all supported build systems:

* **Clojure** — Compiler errors, syntax errors, unresolved symbols
* **Java/Maven** — Compilation errors, missing dependencies
* **Gradle** — Build failures, task errors

## Fallback Behavior

If jbundle cannot parse the error format:

* Full raw output is displayed
* No information is lost
* You see exactly what the underlying tool reported

## Examples

### Clojure Syntax Error

```
error: Unmatched delimiter: )
 --> src/myapp/core.clj:15:1
   |
13 | (defn calculate [x y]
14 |   (+ x y)
15 | ))
   | ^ unexpected closing paren
```

### Java Compilation Error

```
error: cannot find symbol
 --> src/main/java/com/example/App.java:12:9
   |
10 | public void process() {
11 |     List<String> items = new ArrayList<>();
12 |     items.add(123);
   |           ^^^ incompatible types: int cannot be converted to String
13 | }
```

### Missing Dependency

```
error: package org.apache.commons.lang3 does not exist
 --> src/main/java/com/example/Utils.java:3:1
   |
 1 | package com.example;
 2 |
 3 | import org.apache.commons.lang3.StringUtils;
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
```

## Debugging Tips

### Enable Verbose Logging

```bash
RUST_LOG=debug jbundle build --input . --output ./app
```

Shows detailed information about each build step.

### Check Build Tool Output

jbundle runs standard build commands. You can run them manually to debug:

```bash
# Clojure
clojure -T:build uber

# Leiningen
lein uberjar

# Maven
mvn package -DskipTests

# Gradle
gradle build -x test
```

### Common Issues

| Error | Likely Cause |
|-------|--------------|
| "No build system detected" | Missing deps.edn/project.clj/pom.xml/build.gradle |
| "JAR not found" | Build succeeded but no uberjar was created |
| "Main class not found" | MANIFEST.MF missing Main-Class entry |
| "Module not found" | jdeps detected a module that jlink can't resolve |
