mod app;
mod config;
mod fonts;

use anyhow::Result;
use app::{App, Outcome};
use config::Config;

fn main() -> Result<()> {
    let path = config::resolve_config_path()?;
    let mut config = Config::load(path)?;

    if !config.live_config_reload_enabled() {
        eprintln!(
            "warning: general.live_config_reload is not enabled in your Alacritty config; \
             preview will not update live. You can still commit a font choice."
        );
    }

    let families = fonts::list_monospace_families()?;
    if families.is_empty() {
        anyhow::bail!("no monospace fonts found via fc-list");
    }

    let mut terminal = ratatui::init();
    let outcome = App::new(families).run(&mut terminal, &mut config);
    ratatui::restore();

    match outcome? {
        Outcome::Committed(family) => {
            config.apply_family(&family)?;
            config.write()?;
            println!("Set font family to \"{family}\"");
        }
        Outcome::Cancelled => {
            config.restore()?;
            println!("Cancelled, config restored.");
        }
    }

    Ok(())
}
