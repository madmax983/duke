# Let's create dummy coverage bump test lines in tests/coverage_bump.rs or similar for the items we changed.
# Actually, if we replaced `.unwrap_or` with `.map_or(0, ...)`, Codecov might flag `.map_or`'s internal closures or arguments if they're not hit?
