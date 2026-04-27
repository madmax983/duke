Wait, why wasn't the `reachability` module change applied correctly?
Ah! `crates/duke-bytecode/src/lib.rs` was not modified correctly before I submitted, or maybe I reverted it accidentally when I did `git reset HEAD plan.md` followed by `git checkout plan.md`? Oh, `git reset HEAD` was only for `plan.md`, but maybe the `submit` tool committed what was staged? No, the commit successfully submitted and the CI ran against the PR.
Why did `cargo check` pass before?
Because `reachability` was actually NOT private in the final PR!
Let's see the git commit history.
