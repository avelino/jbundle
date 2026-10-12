# Security Policy

## Supported versions

Only the latest tagged release gets security fixes. The `latest` pre-release is a development build from `main` and is not supported.

## Reporting a vulnerability

Please don't open a public issue. Report it privately through GitHub: go to the [Security tab](https://github.com/avelino/jbundle/security/advisories/new) and open a draft advisory.

If that link doesn't work for you, open an issue titled `Security contact request` with no details about the problem, and I'll reach out to you privately.

Include what you found, how to reproduce it and which version you tested. I'll reply as soon as I can and keep you posted until it's fixed.

## Scope

jbundle downloads a JDK, runs your build tool (Clojure CLI, Leiningen, Maven or Gradle) and packs the result into a single binary. Issues in that pipeline, in the release workflow or in `install.sh`/`install.ps1` are in scope. Bugs in the JDK or in your own build are not.
