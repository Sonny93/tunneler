//! Single-flight for the devslot listing: concurrent calls for one host share
//! the `ds ls --json` that is running instead of each running their own.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use tokio::sync::watch;

use crate::devslot::DevslotError;
use crate::runtime::devslot_service::RemoteDevslotListing;
use crate::runtime::engine::EngineInner;

const LISTING_ABORTED_REASON: &str = "the listing task ended before it reported an outcome";

type ListingOutcome = Result<RemoteDevslotListing, DevslotError>;
type ListingReceiver = watch::Receiver<Option<ListingOutcome>>;

/// The listings that are running, by host alias. A listing leaves the map
/// when it ends: nothing is cached.
#[derive(Default)]
pub(crate) struct DevslotListings {
    running: Mutex<HashMap<String, ListingReceiver>>,
}

impl DevslotListings {
    fn running(&self) -> MutexGuard<'_, HashMap<String, ListingReceiver>> {
        self.running.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Removes the entry of a host when dropped, so that a listing that ends in
/// any way (outcome, panic, abort) never leaves a dead entry behind.
struct RunningListing {
    inner: Arc<EngineInner>,
    host: String,
}

impl Drop for RunningListing {
    fn drop(&mut self) {
        self.inner.devslot_listings.running().remove(&self.host);
    }
}

impl EngineInner {
    /// Lists the instances of a host, sharing the listing that is already
    /// running for it when there is one. The listing runs in a task of its
    /// own, so a caller that goes away never cancels it for the others.
    pub(crate) async fn list_devslot(
        self: &Arc<Self>,
        host: &str,
    ) -> Result<RemoteDevslotListing, DevslotError> {
        let mut receiver = self.join_or_start_listing(host);
        let outcome = match receiver.wait_for(Option::is_some).await {
            Ok(finished) => finished.clone(),
            Err(_) => None,
        };
        outcome.unwrap_or_else(|| Err(listing_aborted(host)))
    }

    fn join_or_start_listing(self: &Arc<Self>, host: &str) -> ListingReceiver {
        let mut running = self.devslot_listings.running();
        if let Some(receiver) = running.get(host) {
            return receiver.clone();
        }
        let (sender, receiver) = watch::channel(None);
        running.insert(host.to_owned(), receiver.clone());
        drop(running);
        let entry = RunningListing {
            inner: Arc::clone(self),
            host: host.to_owned(),
        };
        tokio::spawn(async move {
            let outcome = entry.inner.run_devslot_listing(&entry.host).await;
            drop(entry);
            sender.send_replace(Some(outcome));
        });
        receiver
    }
}

fn listing_aborted(host: &str) -> DevslotError {
    DevslotError::Failed {
        host: host.to_owned(),
        status: None,
        stderr: LISTING_ABORTED_REASON.to_owned(),
    }
}
