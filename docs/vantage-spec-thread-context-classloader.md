# Spec: Thread Context ClassLoader

## User Story
As a Developer running a multi-application container, I want threads to have an explicit context ClassLoader (TCCL) attached to them, so that third-party frameworks (like SLF4J or JNDI) can dynamically discover and load application-specific classes without relying on the system classloader.

## The "So What?"
**What business problem does this solve?**
It unblocks execution of complex third-party frameworks like `slf4j-simple`, enabling enterprise-level Java applications to run on Duke. Without this, enterprise libraries crash when they attempt to use `getContextClassLoader` to load custom configurations or bindings.

## Metric Definition
Success = `slf4j-simple` OSS jar test progresses past `java/lang/Thread.getContextClassLoader()Ljava/lang/ClassLoader;`.

## Gap Analysis
Duke currently fails when executing `slf4j-simple` because `java/lang/Thread.getContextClassLoader()Ljava/lang/ClassLoader;` is unsupported or missing.

## Acceptance Criteria
- Must support `java/lang/Thread.getContextClassLoader()Ljava/lang/ClassLoader;`.

## Out of Scope
- Security manager permission checks for `getContextClassLoader()`.
