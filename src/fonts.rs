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
        // Each line can list comma-separated aliases for the same font file, e.g.
        // "IBM Plex Mono,IBM Plex Mono ExtraLight" — the later names are per-weight
        // aliases that fontconfig only style-links loosely (often reporting a bogus
        // "Regular" style alongside the real one), so an exact family+style lookup
        // against them fails. Only the first name is the true family; use that.
        let Some(family) = line.split(',').next() else {
            continue;
        };
        let family = family.trim();
        if !family.is_empty() && is_usable(family) {
            families.insert(family.to_string());
        }
    }

    Ok(families.into_iter().collect())
}

/// Filter out macOS's private system-UI faces (leading dot, e.g. ".SF NS Mono",
/// ".LastResort") and emoji faces, none of which are meaningful terminal text fonts.
fn is_usable(family: &str) -> bool {
    !family.starts_with('.') && !family.to_lowercase().contains("emoji")
}
