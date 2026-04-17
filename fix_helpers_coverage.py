with open("crates/duke-telemetry/src/helpers.rs", "r") as f:
    content = f.read()

# Add #[cfg(not(tarpaulin_include))] to all the helpers functions since tarpaulin thinks they are not covered.
# Oh, it's actually just `#[cfg_attr(tarpaulin, ignore)]` or `#[tarpaulin::skip]`
# Wait, let's just make the whole module ignored for tarpaulin, or just test it correctly.
# Why is it not covered? Because we run tests inside `mod tests` under `#[cfg(test)]`.
# The function `site3` etc are used by tests, but tarpaulin doesn't see them as covered?
# Wait, I changed the `mod tests` to have `#[cfg(all(test, feature = "telemetry"))]`.
# When I ran tarpaulin with `--out Lcov` it didn't pass the `--all-features` flag!
# Let me re-run tarpaulin with `--all-features`!
