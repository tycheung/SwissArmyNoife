# Kotlin SDK (`sak334`)

**Decision (sak334-a):** ship a thin **idiomatic Kotlin JVM client** under this tree (Gradle Kotlin
DSL), not Java-JAR interop-only. Consumers who only need JVM HTTP can still use
`sdks/java` (`com.swissarmynoife:swissarmynoife-sdk`) from Kotlin without this module.

## Build / test

Requires JDK 17+.

```bash
cd SwissArmyNoife/sdks/kotlin
./gradlew test
```

On Windows without the wrapper yet: use a local Gradle 8+ (`gradle test`).

Scaffold (`sak334-a`). Client surfaces land in `sak334-b` / `sak334-c`.
