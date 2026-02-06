# JAR Analysis

Inspect your JAR before packaging to understand what's inside, find optimization opportunities, and catch potential issues.

## Usage

```bash
# Analyze current project (builds uberjar first)
jbundle analyze

# Analyze a pre-built JAR directly
jbundle analyze --input ./target/app-standalone.jar
```

## What It Reports

### Category Breakdown

Every entry in the JAR is classified into one of:

| Category | Matches |
|----------|---------|
| Classes | `*.class` |
| Clojure sources | `*.clj`, `*.cljc`, `*.cljs` |
| Java sources | `*.java` |
| Native libs | `*.so`, `*.dylib`, `*.dll`, `*.jnilib` |
| Metadata | `META-INF/*` (non-class) |
| Resources | Everything else |

### Top Packages

Entries are grouped by the first 3 path segments (matching Maven groupId convention). For example, `org/apache/commons/lang3/StringUtils.class` maps to `org.apache.commons`.

### Clojure Namespaces

Detected from `__init.class` entries. For example, `myapp/core__init.class` maps to namespace `myapp.core`.

### Shrink Estimate

Shows how much space `--shrink` would save by removing non-essential files (Maven metadata, JAR signatures, Java source files, build tool artifacts).

### Potential Issues

- **Duplicate classes** — Same class path appearing multiple times (common in uberjars with dependency conflicts)
- **Large resources** — Files over 1 MB that may be worth reviewing (embedded models, datasets, etc.)

## Example Output

```
JAR: target/app-standalone.jar (87.3 MB)
Entries: 12,345

Category              Size       %    Files
────────────────────────────────────────────────
Classes           42.1 MB    48%    8,432
Resources         38.7 MB    44%    1,203
Native libs        5.2 MB     6%       12
Metadata           1.3 MB     2%      806

Top packages by size:
  org.apache.poi                       28.4 MB  1,322 files
  com.google.guava                      3.1 MB    456 files
  org.clojure                           2.8 MB    342 files

Clojure namespaces:
  clojure.core                          2.1 MB    342 files
  myapp.handlers                        0.5 MB     28 files

Estimated --shrink savings: 12.4 MB (14%) — 892 removable files

Potential issues:
  Duplicate class: javax/servlet/Servlet.class (3 occurrences)
  Large resource: data/model.bin (8.5 MB)
```

## When to Use

- **Before first build** — Understand your JAR composition and spot bloat
- **Evaluating --shrink** — See the savings estimate before enabling it
- **Debugging binary size** — Find which dependencies are largest
- **Dependency conflicts** — Detect duplicate classes from overlapping dependencies
