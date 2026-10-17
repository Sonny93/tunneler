use std::sync::{Arc, OnceLock};

use super::cookie::FakeCookie;
use super::display::LocalDisplay;
use super::error::X11Error;

/// The local side of X11 forwarding: where the X server is and the cookie
/// that grants access to it.
pub(crate) struct LocalX11 {
    pub(crate) display: LocalDisplay,
    pub(crate) real_cookie: Option<Vec<u8>>,
}

/// What an SSH session needs to serve the X11 channels the server opens.
pub(crate) struct X11Forwarding {
    pub(crate) fake_cookie: FakeCookie,
    pub(crate) local: LocalX11,
}

/// Decides, for one SSH session, whether the X11 channels the server opens
/// are served.
///
/// Until X11 is enabled every channel is refused, so that a server cannot
/// reach the local X server on its own initiative. Enabling happens once per
/// session: the first display and cookie stay for the whole session.
#[derive(Default)]
pub(crate) struct X11Router {
    forwarding: OnceLock<Arc<X11Forwarding>>,
}

impl X11Router {
    /// Starts serving X11 channels with `local` unless the session already
    /// does, and returns the forwarding in force.
    pub(crate) fn enable(&self, local: LocalX11) -> Result<Arc<X11Forwarding>, X11Error> {
        if let Some(existing) = self.forwarding.get() {
            return Ok(Arc::clone(existing));
        }
        let fake_cookie = FakeCookie::generate()?;
        let forwarding = self
            .forwarding
            .get_or_init(|| Arc::new(X11Forwarding { fake_cookie, local }));
        Ok(Arc::clone(forwarding))
    }

    /// The forwarding to serve a channel with; `None` while X11 is not enabled.
    pub(crate) fn route(&self) -> Option<Arc<X11Forwarding>> {
        self.forwarding.get().map(Arc::clone)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::x11::display::DisplayEndpoint;

    fn local(number: u16) -> LocalX11 {
        LocalX11 {
            display: LocalDisplay {
                number,
                endpoint: DisplayEndpoint::UnixSocket(PathBuf::from("/tmp/.X11-unix/X0")),
            },
            real_cookie: None,
        }
    }

    #[test]
    fn should_not_route_when_x11_is_not_enabled() {
        assert!(X11Router::default().route().is_none());
    }

    #[test]
    fn should_route_when_x11_is_enabled() {
        let router = X11Router::default();
        router.enable(local(0)).unwrap();

        assert!(router.route().is_some());
    }

    #[test]
    fn should_keep_the_first_forwarding_when_x11_is_enabled_twice() {
        let router = X11Router::default();
        let first = router.enable(local(0)).unwrap();

        let second = router.enable(local(7)).unwrap();

        assert!(Arc::ptr_eq(&first, &second));
    }

    #[test]
    fn should_keep_the_first_display_when_x11_is_enabled_twice() {
        let router = X11Router::default();
        router.enable(local(0)).unwrap();
        router.enable(local(7)).unwrap();

        assert_eq!(router.route().unwrap().local.display.number, 0);
    }
}
