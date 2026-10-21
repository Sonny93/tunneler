use std::collections::VecDeque;
use std::future::pending;
use std::net::SocketAddr;
use std::time::Duration;

use tokio::net::TcpStream;
use tokio::time::{Instant, sleep_until};

use crate::runtime::forward_run::ClientProtocol;

/// How many client connections a tunnel holds while it opens its SSH session;
/// the ones beyond are closed.
pub(crate) const HELD_CONNECTIONS_LIMIT: usize = 32;

/// A client that connected while the tunnel had no session.
pub(crate) struct HeldClient {
    pub(crate) client: TcpStream,
    pub(crate) peer: SocketAddr,
    pub(crate) protocol: ClientProtocol,
}

/// A first-in first-out queue that refuses what does not fit.
pub(crate) struct BoundedQueue<Item> {
    items: VecDeque<Item>,
    capacity: usize,
}

impl<Item> BoundedQueue<Item> {
    pub(crate) fn new(capacity: usize) -> Self {
        Self {
            items: VecDeque::new(),
            capacity,
        }
    }

    /// Queues `item`, or gives it back when the queue is full.
    pub(crate) fn push(&mut self, item: Item) -> Result<(), Item> {
        if self.items.len() >= self.capacity {
            return Err(item);
        }
        self.items.push_back(item);
        Ok(())
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Takes every queued item, oldest first.
    pub(crate) fn take_all(&mut self) -> Vec<Item> {
        self.items.drain(..).collect()
    }
}

/// Tells when a tunnel has had no connection for long enough to give its
/// session back.
pub(crate) struct IdleTimer {
    limit: Duration,
    idle_since: Option<Instant>,
}

impl IdleTimer {
    pub(crate) fn new(limit: Duration) -> Self {
        Self {
            limit,
            idle_since: None,
        }
    }

    /// Records whether the tunnel carries a connection right now. The wait
    /// starts when the tunnel becomes idle and is kept while it stays so.
    pub(crate) fn observe(&mut self, has_connections: bool) {
        if has_connections {
            self.idle_since = None;
            return;
        }
        self.idle_since.get_or_insert_with(Instant::now);
    }

    /// Resolves once the tunnel has been idle for the limit; never while it
    /// carries a connection.
    pub(crate) async fn expired(&self) {
        match self.idle_since {
            Some(since) => sleep_until(since + self.limit).await,
            None => pending().await,
        }
    }
}

#[cfg(test)]
mod tests {
    use tokio::time::{advance, timeout};

    use super::*;

    const LIMIT: Duration = Duration::from_secs(10);
    const ALMOST_LIMIT: Duration = Duration::from_secs(9);
    const ONE_SECOND: Duration = Duration::from_secs(1);

    #[test]
    fn should_refuse_the_item_when_the_queue_is_full() {
        let mut queue = BoundedQueue::new(1);
        queue.push("first").unwrap();

        assert_eq!(queue.push("second"), Err("second"));
    }

    #[test]
    fn should_give_the_items_back_in_order_when_taking_all() {
        let mut queue = BoundedQueue::new(3);
        queue.push(1).unwrap();
        queue.push(2).unwrap();

        assert_eq!(queue.take_all(), vec![1, 2]);
    }

    #[test]
    fn should_be_empty_when_everything_was_taken() {
        let mut queue = BoundedQueue::new(3);
        queue.push(1).unwrap();
        queue.take_all();

        assert!(queue.is_empty());
    }

    #[tokio::test(start_paused = true)]
    async fn should_expire_when_the_tunnel_stays_idle_for_the_limit() {
        let mut timer = IdleTimer::new(LIMIT);
        timer.observe(false);

        advance(LIMIT).await;

        assert!(
            timeout(Duration::from_millis(1), timer.expired())
                .await
                .is_ok()
        );
    }

    #[tokio::test(start_paused = true)]
    async fn should_not_expire_before_the_limit() {
        let mut timer = IdleTimer::new(LIMIT);
        timer.observe(false);

        advance(ALMOST_LIMIT).await;

        assert!(
            timeout(Duration::from_millis(1), timer.expired())
                .await
                .is_err()
        );
    }

    #[tokio::test(start_paused = true)]
    async fn should_never_expire_while_a_connection_is_open() {
        let mut timer = IdleTimer::new(LIMIT);
        timer.observe(true);

        advance(LIMIT * 3).await;

        assert!(
            timeout(Duration::from_millis(1), timer.expired())
                .await
                .is_err()
        );
    }

    #[tokio::test(start_paused = true)]
    async fn should_restart_the_wait_when_a_connection_comes_and_goes() {
        let mut timer = IdleTimer::new(LIMIT);
        timer.observe(false);
        advance(ALMOST_LIMIT).await;
        timer.observe(true);
        timer.observe(false);

        advance(ALMOST_LIMIT).await;

        assert!(
            timeout(Duration::from_millis(1), timer.expired())
                .await
                .is_err()
        );
    }

    #[tokio::test(start_paused = true)]
    async fn should_keep_the_original_start_when_idleness_is_observed_again() {
        let mut timer = IdleTimer::new(LIMIT);
        timer.observe(false);
        advance(ALMOST_LIMIT).await;
        timer.observe(false);

        advance(ONE_SECOND).await;

        assert!(
            timeout(Duration::from_millis(1), timer.expired())
                .await
                .is_ok()
        );
    }
}
