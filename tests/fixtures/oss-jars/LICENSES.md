# Vendored OSS JARs

These JARs are vendored for hermetic, offline smoke tests.

| JAR | Version | License | Upstream | SHA-256 |
| --- | --- | --- | --- | --- |
| `slf4j-api-2.0.13.jar` | `2.0.13` | `MIT` | <https://repo1.maven.org/maven2/org/slf4j/slf4j-api/2.0.13/slf4j-api-2.0.13.jar> | `e7c2a48e8515ba1f49fa637d57b4e2f590b3f5bd97407ac699c3aa5efb1204a9` |
| `slf4j-simple-2.0.13.jar` | `2.0.13` | `MIT` | <https://repo1.maven.org/maven2/org/slf4j/slf4j-simple/2.0.13/slf4j-simple-2.0.13.jar> | `3153fe1d689cffb94f1530b58470c306685ba68844de8857116e3b6ebb81d9f7` |
| `gson-2.11.0.jar` | `2.11.0` | `Apache-2.0` | <https://repo1.maven.org/maven2/com/google/code/gson/gson/2.11.0/gson-2.11.0.jar> | `57928d6e5a6edeb2abd3770a8f95ba44dce45f3b23b7a9dc2b309c581552a78b` |
| `commons-lang3-3.17.0.jar` | `3.17.0` | `Apache-2.0` | <https://repo1.maven.org/maven2/org/apache/commons/commons-lang3/3.17.0/commons-lang3-3.17.0.jar> | `6ee731df5c8e5a2976a1ca023b6bb320ea8d3539fbe64c8a1d5cb765127c33b4` |
| `spring-boot/duke-spring-boot-app-3.5.12.jar` | `3.5.12` | `Apache-2.0` | Self-built Spring Boot fat jar (bundled Spring/loader code is Apache-2.0); recipe in `spring-boot/README.md`; parent `org.springframework.boot:spring-boot-starter-parent:3.5.12` | `008bd724f19b5df6ba56b98655e389ce811942638d84c6f9952e916804c80366` |
| `spring-boot/duke-spring-boot-ladder-3.5.12.jar` | `3.5.12` | `Apache-2.0` | Self-built Spring-Boot-layout fat jar (loader from `spring-boot-loader-3.5.12.jar`, bundles <https://repo1.maven.org/maven2/commons-logging/commons-logging/1.3.5/commons-logging-1.3.5.jar>); recipe in `spring-boot/README.md` | `162689fc62f024257fc8b3c10cc091f814eb975ca9c6725d5f3c96d37881370a` |
