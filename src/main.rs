use cosmic::cosmic_config::{self, CosmicConfigEntry};

use crate::app::AppState;
use crate::config::{CONFIG_VERSION, Config};
use crate::localize::localize;

#[macro_use]
extern crate tracing;

mod app;
mod brightness;
#[cfg(feature = "brightness-sync-daemon")]
mod daemon;
#[cfg(feature = "brightness-sync-daemon")]
mod ui_sync;
mod config;
#[cfg(feature = "apple-hid-displays")]
mod devices;
mod error;
mod hotplug;
mod icon;
mod localize;
mod monitor;
mod permissions;
mod protocols;
mod randr;
mod view;

fn setup_logs() {
    use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

    let fmt_layer = fmt::layer().with_target(true);  // Enable target to see where logs come from
    // Filter out noisy DDC/CI errors from the ddc_hi library
    // These transient errors are normal and handled by our retry logic
    let filter_layer = EnvFilter::try_from_default_env().unwrap_or(EnvFilter::new(format!(
        "error,{}=info",
        env!("CARGO_CRATE_NAME")
    )));

    if let Ok(journal_layer) = tracing_journald::layer() {
        tracing_subscriber::registry()
            .with(filter_layer)
            .with(fmt_layer)
            .with(journal_layer)
            .init();
    } else {
        tracing_subscriber::registry()
            .with(filter_layer)
            .with(fmt_layer)
            .init();
    }

    // Bridge log crate to tracing AFTER tracing is initialized
    // This allows us to filter ddc_hi library logs
    tracing_log::LogTracer::init().ok();
}

fn main() -> cosmic::iced::Result {
    setup_logs();
    localize();

    let (config_handler, config) = match cosmic_config::Config::new(app::APPID, CONFIG_VERSION) {
        Ok(config_handler) => {
            let config = match Config::get_entry(&config_handler) {
                Ok(ok) => ok,
                Err((errs, config)) => {
                    error!("errors loading config: {:?}", errs);
                    config
                }
            };
            (Some(config_handler), config)
        }
        Err(err) => {
            error!("failed to create config handler: {}", err);
            (None, Config::default())
        }
    };

    // Check for old config format and log migration warning
    config.check_migration_needed();

    cosmic::applet::run::<AppState>((config_handler, config))
}
