use anyhow::{Context, Result};
use std::collections::BTreeSet;
use std::process::Command;

/// List installed monospace font families via fontconfig, deduped and sorted.
pub fn list_monospace_families() -> Result<Vec<String>> {
    let output = Command::new("fc-list")
        .args([":spacing=100", "family"])
        .output()
        .context("failed to run fc-list (is fontconfig installed?)")?;

    if !output.status.success() {
        anyhow::bail!(
            "fc-list exited with status {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut families: BTreeSet<String> = BTreeSet::new();
    for line in stdout.lines() {
        for family in line.split(',') {
            let family = family.trim();
            if !family.is_empty() {
                families.insert(family.to_string());
            }
        }
    }

    Ok(families.into_iter().collect())
}
