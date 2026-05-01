# ADR 002: Regex Backreferences Remain Unsupported

## Status

Accepted

## Context

Duke implements `java.util.regex.Pattern` and `Matcher` with Rust's `regex` crate. That engine deliberately rejects backreferences such as `(\\w+)\\s+\\1` because they are not regular-language constructs.

## Decision

Duke will fail such patterns at compile time with a catchable `java/util/regex/PatternSyntaxException`. It must not panic, return `VmError::Unsupported`, or silently produce a wrong answer.

## Consequences

Fixtures that exercise backreferences assert the documented exception path. Supporting HotSpot-compatible backreferences requires a future regex engine change rather than a translator tweak.
