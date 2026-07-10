# Spring Boot fat-jar fixtures

Two **real**, boot-verified Spring Boot 3.5.x layout fat jars for exercising
`duke -jar` routing through the Spring Boot `JarLauncher`
(`org.springframework.boot.loader.launch.JarLauncher`, resolved against the
repo-root `spring-boot-loader-3.5.12.jar` loader classes). Both were confirmed to
boot on a real JVM (OpenJDK 21.0.10) before vendoring.

| Fixture | Rung | Size | Dependency surface |
| --- | --- | --- | --- |
| `duke-spring-boot-ladder-3.5.12.jar` | intermediate | 277 KB | commons-logging only |
| `duke-spring-boot-app-3.5.12.jar` | full | 10.6 MB | full `spring-boot-starter` |

Both use `Main-Class: org.springframework.boot.loader.launch.JarLauncher` with a
`Start-Class` app, nested libs STORED (uncompressed) under `BOOT-INF/lib/`, and a
`BOOT-INF/classpath.idx`. SHA-256 and license are recorded in `../LICENSES.md`.

## A — `duke-spring-boot-app-3.5.12.jar` (full framework)

A genuine `@SpringBootApplication` (`com.example.duke.DukeApplication`) that calls
`SpringApplication.run(...)` and a `CommandLineRunner` bean. Prints the Spring
banner and `Started DukeApplication in ... seconds`, then exits (no web server).

Built with **Maven 3.9.11** using `spring-boot-starter-parent:3.5.12` and the
`spring-boot-maven-plugin` `repackage` goal.

`pom.xml`:

```xml
<project xmlns="http://maven.apache.org/POM/4.0.0"
         xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
         xsi:schemaLocation="http://maven.apache.org/POM/4.0.0 http://maven.apache.org/xsd/maven-4.0.0.xsd">
    <modelVersion>4.0.0</modelVersion>
    <parent>
        <groupId>org.springframework.boot</groupId>
        <artifactId>spring-boot-starter-parent</artifactId>
        <version>3.5.12</version>
        <relativePath/>
    </parent>
    <groupId>com.example.duke</groupId>
    <artifactId>duke-spring-boot-app</artifactId>
    <version>3.5.12</version>
    <packaging>jar</packaging>
    <properties><java.version>21</java.version></properties>
    <dependencies>
        <dependency>
            <groupId>org.springframework.boot</groupId>
            <artifactId>spring-boot-starter</artifactId>
        </dependency>
    </dependencies>
    <build>
        <plugins>
            <plugin>
                <groupId>org.springframework.boot</groupId>
                <artifactId>spring-boot-maven-plugin</artifactId>
            </plugin>
        </plugins>
    </build>
</project>
```

`src/main/java/com/example/duke/DukeApplication.java`:

```java
package com.example.duke;

import org.springframework.boot.CommandLineRunner;
import org.springframework.boot.SpringApplication;
import org.springframework.boot.autoconfigure.SpringBootApplication;
import org.springframework.context.annotation.Bean;

@SpringBootApplication
public class DukeApplication {
    public static void main(String[] args) {
        SpringApplication.run(DukeApplication.class, args);
    }
    @Bean
    public CommandLineRunner runner() {
        return args -> System.out.println("Duke Spring Boot fixture application started successfully.");
    }
}
```

Build:

```sh
mvn -q -B package
# -> target/duke-spring-boot-app-3.5.12.jar
```

## B — `duke-spring-boot-ladder-3.5.12.jar` (intermediate rung)

A small `com.example.duke.ladder.LadderApplication` (plain `main`, no Spring
framework) that exercises "Spring-like" behaviour with a far smaller dependency
surface, so it goes through the **same** `JarLauncher` routing as fixture A:

- commons-logging (`org.apache.commons.logging.LogFactory` / `Log`) logging
- classpath resource scanning: `getResourceAsStream` + `ClassLoader.getResources`
- reflection: `Class.forName` + `Method.invoke`
- collections / streams

Only dependency: `commons-logging:1.3.5` (from Maven Central), bundled STORED in
`BOOT-INF/lib/`.

Manual assembly recipe:

```sh
# app source in src/com/example/duke/ladder/LadderApplication.java
# resources in classes/META-INF/duke-ladder.properties, classes/com/example/duke/ladder/greeting.txt
LOADER=spring-boot-loader-3.5.12.jar   # repo root

javac -classpath commons-logging-1.3.5.jar -d classes \
      src/com/example/duke/ladder/LadderApplication.java

rm -rf stage && mkdir -p stage/BOOT-INF/classes stage/BOOT-INF/lib stage/META-INF
( cd stage && unzip -q "../$LOADER" 'org/*' )          # loader classes at root
cp -r classes/. stage/BOOT-INF/classes/
cp commons-logging-1.3.5.jar stage/BOOT-INF/lib/
printf -- '- "BOOT-INF/lib/commons-logging-1.3.5.jar"\n' > stage/BOOT-INF/classpath.idx

cat > stage/META-INF/MANIFEST.MF <<'MF'
Manifest-Version: 1.0
Created-By: duke test fixture builder
Main-Class: org.springframework.boot.loader.launch.JarLauncher
Start-Class: com.example.duke.ladder.LadderApplication
Spring-Boot-Version: 3.5.12
Spring-Boot-Classes: BOOT-INF/classes/
Spring-Boot-Lib: BOOT-INF/lib/
Spring-Boot-Classpath-Index: BOOT-INF/classpath.idx
MF

OUT=duke-spring-boot-ladder-3.5.12.jar
( cd stage
  jar --create --file "../$OUT" --manifest META-INF/MANIFEST.MF -C . org
  zip -qr "../$OUT" BOOT-INF/classes BOOT-INF/classpath.idx
  zip -q0 "../$OUT" BOOT-INF/lib/commons-logging-1.3.5.jar )   # STORED nested jar
```

`commons-logging-1.3.5.jar` SHA-256:
`6d7a744e4027649fbb50895df9497d109f98c766a637062fe8d2eabbb3140ba4`
(<https://repo1.maven.org/maven2/commons-logging/commons-logging/1.3.5/commons-logging-1.3.5.jar>).

## Verifying they are real

```sh
java -jar duke-spring-boot-app-3.5.12.jar      # prints Spring banner + "Started ..."
java -jar duke-spring-boot-ladder-3.5.12.jar   # prints JCL log lines for each rung
```
