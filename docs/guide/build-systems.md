# Build Systems

jbundle automatically detects your build system and runs the appropriate commands.

## Supported Build Systems

| Build System | Detection File | Build Command |
|--------------|----------------|---------------|
| Clojure (tools.build) | `deps.edn` | `clojure -T:build uber` |
| Leiningen | `project.clj` | `lein uberjar` |
| Maven | `pom.xml` | `mvn package -DskipTests` |
| Gradle | `build.gradle(.kts)` | `gradle build -x test` |

## Clojure (deps.edn)

### Requirements

Your `deps.edn` must have a `:build` alias with `tools.build`:

```clojure
{:deps {...}
 :aliases
 {:build
  {:deps {io.github.clojure/tools.build {:mvn/version "0.10.5"}}
   :ns-default build}}}
```

And a `build.clj` with an `uber` function:

```clojure
(ns build
  (:require [clojure.tools.build.api :as b]))

(def lib 'com.example/my-app)
(def version "1.0.0")
(def class-dir "target/classes")
(def uber-file (format "target/%s-%s.jar" (name lib) version))

(defn uber [_]
  (b/copy-dir {:src-dirs ["src" "resources"]
               :target-dir class-dir})
  (b/compile-clj {:basis (b/create-basis {:project "deps.edn"})
                  :class-dir class-dir})
  (b/uber {:class-dir class-dir
           :uber-file uber-file
           :basis (b/create-basis {:project "deps.edn"})
           :main 'com.example.main}))
```

### What jbundle Does

```bash
clojure -T:build uber
# Then looks for JAR in target/
```

## Clojure (Leiningen)

### Requirements

Your `project.clj` should specify a `:main` namespace:

```clojure
(defproject my-app "1.0.0"
  :dependencies [[org.clojure/clojure "1.11.1"]]
  :main my-app.core
  :aot :all)
```

### What jbundle Does

```bash
lein uberjar
# Then looks for *-standalone.jar in target/
```

## Java (Maven)

### Requirements

Configure the Maven Shade Plugin for uberjar creation:

```xml
<build>
  <plugins>
    <plugin>
      <groupId>org.apache.maven.plugins</groupId>
      <artifactId>maven-shade-plugin</artifactId>
      <version>3.5.1</version>
      <executions>
        <execution>
          <phase>package</phase>
          <goals><goal>shade</goal></goals>
          <configuration>
            <transformers>
              <transformer implementation="org.apache.maven.plugins.shade.resource.ManifestResourceTransformer">
                <mainClass>com.example.Main</mainClass>
              </transformer>
            </transformers>
          </configuration>
        </execution>
      </executions>
    </plugin>
  </plugins>
</build>
```

### What jbundle Does

```bash
mvn package -DskipTests
# Then looks for JAR in target/
```

## Java (Gradle)

### Requirements

Use the Shadow plugin for uberjar creation:

```kotlin
// build.gradle.kts
plugins {
    application
    id("com.github.johnrengelman.shadow") version "8.1.1"
}

application {
    mainClass.set("com.example.Main")
}
```

Or with Groovy DSL:

```groovy
// build.gradle
plugins {
    id 'application'
    id 'com.github.johnrengelman.shadow' version '8.1.1'
}

application {
    mainClass = 'com.example.Main'
}
```

### What jbundle Does

```bash
gradle build -x test
# Then looks for *-all.jar in build/libs/
```

## From Pre-built JAR

Skip the build step entirely:

```bash
jbundle build --input ./target/app.jar --output ./dist/app
```

Useful when:
* You have a custom build process
* The JAR is built by CI
* You're testing with an existing artifact

## Detection Priority

If multiple build files exist, jbundle uses this priority:

1. `deps.edn` (Clojure tools.build)
2. `project.clj` (Leiningen)
3. `pom.xml` (Maven)
4. `build.gradle` or `build.gradle.kts` (Gradle)

## Troubleshooting

### "No build system detected"

Ensure one of the supported build files is in the root of `--input` directory.

### "JAR not found after build"

Check that your build produces an uberjar (not just a thin JAR). The JAR must include all dependencies.

### "Main class not found"

Ensure your JAR's `MANIFEST.MF` specifies `Main-Class`.
