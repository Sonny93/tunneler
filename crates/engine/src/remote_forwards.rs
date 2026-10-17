use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use russh::Channel;
use russh::client::{ChannelOpenHandle, Msg};
use tokio::sync::mpsc;

/// A connection the server opened for a remote forward. The tunnel that owns
/// the forward accepts it once the local target answers, or refuses it by
/// dropping or rejecting `reply`.
pub(crate) struct ForwardedConnection {
    pub(crate) channel: Channel<Msg>,
    pub(crate) reply: ChannelOpenHandle,
    pub(crate) originator: String,
}

/// Where the connections of one remote forward are sent.
pub(crate) type ForwardedSender = mpsc::UnboundedSender<ForwardedConnection>;

struct Entry {
    id: u64,
    address: String,
    port: u16,
    sender: ForwardedSender,
}

#[derive(Default)]
struct Entries {
    next_id: u64,
    list: Vec<Entry>,
}

/// The remote forwards of one SSH session, so that the connections the
/// server opens reach the tunnel that asked for them.
///
/// A connection is routed by the port the server reports. When several
/// forwards of the session share that port, the reported address breaks the
/// tie (compared ignoring case); when it matches none of them the connection
/// is not routed. Servers do not agree on how they report the address of a
/// forward bound to a name such as `localhost`, so it is never required when
/// the port alone is unambiguous.
#[derive(Default)]
pub(crate) struct RemoteForwardRegistry {
    entries: Mutex<Entries>,
}

impl RemoteForwardRegistry {
    fn lock(&self) -> MutexGuard<'_, Entries> {
        self.entries.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Routes the connections of the forward on `address:port` to `sender`
    /// until the returned registration is dropped.
    pub(crate) fn register(
        self: &Arc<Self>,
        address: &str,
        port: u16,
        sender: ForwardedSender,
    ) -> RemoteForwardRegistration {
        let mut entries = self.lock();
        let id = entries.next_id;
        entries.next_id += 1;
        entries.list.push(Entry {
            id,
            address: address.to_owned(),
            port,
            sender,
        });
        RemoteForwardRegistration {
            registry: Arc::clone(self),
            id,
            address: address.to_owned(),
            port,
        }
    }

    /// The tunnel that owns the connection the server reports on
    /// `connected_address:connected_port`.
    pub(crate) fn route(
        &self,
        connected_address: &str,
        connected_port: u32,
    ) -> Option<ForwardedSender> {
        let entries = self.lock();
        let mut candidates = entries
            .list
            .iter()
            .filter(|entry| u32::from(entry.port) == connected_port);
        let first = candidates.next()?;
        let Some(second) = candidates.next() else {
            return Some(first.sender.clone());
        };
        [first, second]
            .into_iter()
            .chain(candidates)
            .find(|entry| entry.address.eq_ignore_ascii_case(connected_address))
            .map(|entry| entry.sender.clone())
    }

    fn unregister(&self, id: u64) {
        self.lock().list.retain(|entry| entry.id != id);
    }
}

/// The claim of a tunnel on a remote forward; dropping it stops the routing.
pub(crate) struct RemoteForwardRegistration {
    registry: Arc<RemoteForwardRegistry>,
    id: u64,
    address: String,
    port: u16,
}

impl RemoteForwardRegistration {
    /// The address the server was asked to listen on.
    pub(crate) fn address(&self) -> &str {
        &self.address
    }

    /// The port the server was asked to listen on.
    pub(crate) fn port(&self) -> u16 {
        self.port
    }
}

impl Drop for RemoteForwardRegistration {
    fn drop(&mut self) {
        self.registry.unregister(self.id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry() -> Arc<RemoteForwardRegistry> {
        Arc::new(RemoteForwardRegistry::default())
    }

    fn sender() -> (
        ForwardedSender,
        mpsc::UnboundedReceiver<ForwardedConnection>,
    ) {
        mpsc::unbounded_channel()
    }

    #[test]
    fn should_route_to_the_forward_when_the_port_matches() {
        let registry = registry();
        let (forward, _receiver) = sender();
        let _registration = registry.register("localhost", 8080, forward);

        assert!(registry.route("127.0.0.1", 8080).is_some());
    }

    #[test]
    fn should_not_route_when_no_forward_uses_the_port() {
        let registry = registry();
        let (forward, _receiver) = sender();
        let _registration = registry.register("localhost", 8080, forward);

        assert!(registry.route("localhost", 9090).is_none());
    }

    #[test]
    fn should_not_route_when_the_forward_was_unregistered() {
        let registry = registry();
        let (forward, _receiver) = sender();
        drop(registry.register("localhost", 8080, forward));

        assert!(registry.route("localhost", 8080).is_none());
    }

    #[test]
    fn should_route_by_address_when_several_forwards_share_the_port() {
        let registry = registry();
        let (first, _first_receiver) = sender();
        let (second, _second_receiver) = sender();
        let _first = registry.register("127.0.0.1", 8080, first);
        let _second = registry.register("198.51.100.10", 8080, second.clone());

        let routed = registry.route("198.51.100.10", 8080).unwrap();

        assert!(routed.same_channel(&second));
    }

    #[test]
    fn should_compare_addresses_ignoring_case_when_several_forwards_share_the_port() {
        let registry = registry();
        let (first, _first_receiver) = sender();
        let (second, _second_receiver) = sender();
        let _first = registry.register("localhost", 8080, first);
        let _second = registry.register("Example.COM", 8080, second);

        assert!(registry.route("example.com", 8080).is_some());
    }

    #[test]
    fn should_not_route_when_several_forwards_share_the_port_and_no_address_matches() {
        let registry = registry();
        let (first, _first_receiver) = sender();
        let (second, _second_receiver) = sender();
        let _first = registry.register("127.0.0.1", 8080, first);
        let _second = registry.register("198.51.100.10", 8080, second);

        assert!(registry.route("203.0.113.10", 8080).is_none());
    }

    #[test]
    fn should_report_the_address_and_port_of_a_registration() {
        let registry = registry();
        let (forward, _receiver) = sender();

        let registration = registry.register("localhost", 8080, forward);

        assert_eq!(
            (registration.address(), registration.port()),
            ("localhost", 8080)
        );
    }
}
