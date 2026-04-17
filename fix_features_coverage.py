with open("crates/duke-telemetry/src/lib.rs", "r") as f:
    content = f.read()

content = content.replace(
"""    pub fn print_report(&self, w: &mut dyn std::io::Write) -> std::io::Result<()> {""",
"""    #[cfg(feature = "telemetry")]
    pub fn print_report(&self, w: &mut dyn std::io::Write) -> std::io::Result<()> {""")

content = content.replace(
"""    pub fn to_markdown_report(&self) -> String {""",
"""    #[cfg(feature = "telemetry")]
    pub fn to_markdown_report(&self) -> String {""")

with open("crates/duke-telemetry/src/lib.rs", "w") as f:
    f.write(content)
