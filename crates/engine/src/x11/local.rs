use std::sync::Arc;

use tokio::sync::watch;
use tracing::debug;

use super::display::resolve_display;
use super::environment::X11Environment;
use super::error::X11Error;
use super::router::LocalX11;
use super::xauthority::load_cookie;
use crate::blocking::run_blocking;
use crate::store::{Settings, Store};

/// Finds the local X display of a host and the cookie that opens it.
pub(crate) struct LocalX11Resolver {
    store: Arc<Store>,
    settings: watch::Receiver<Settings>,
    environment: Arc<dyn X11Environment>,
}

impl LocalX11Resolver {
    pub(crate) fn new(
        store: Arc<Store>,
        settings: watch::Receiver<Settings>,
        environment: Arc<dyn X11Environment>,
    ) -> Self {
        Self {
            store,
            settings,
            environment,
        }
    }

    /// The display for `host`: its override, else the settings, else
    /// `$DISPLAY`, and the cookie the `Xauthority` file holds for it.
    pub(crate) async fn resolve(&self, host: &str) -> Result<LocalX11, X11Error> {
        let host_override = self
            .store
            .document()
            .hosts
            .get(host)
            .and_then(|overrides| overrides.x11_display.clone());
        let settings_display = self.settings.borrow().x11_display.clone();
        let local_display = resolve_display(
            host_override.as_deref(),
            settings_display.as_deref(),
            self.environment.as_ref(),
        )?;
        let environment = Arc::clone(&self.environment);
        let lookup_display = local_display.clone();
        let real_cookie =
            run_blocking(move || load_cookie(&lookup_display, environment.as_ref())).await?;
        debug!(%host, display = %local_display, has_cookie = real_cookie.is_some(), "resolved the local X display");
        Ok(LocalX11 {
            display: local_display,
            real_cookie,
        })
    }
}
