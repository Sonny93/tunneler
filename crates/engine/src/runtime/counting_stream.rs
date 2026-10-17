use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

use crate::runtime::metrics::MetricsCounters;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Side {
    Client,
    Target,
}

/// A local socket of a forwarded connection, counting what passes through.
///
/// Bytes that go from the clients to the target are counted as out, bytes
/// that come back as in, whichever end of the connection the socket is.
pub(crate) struct CountingStream<Stream> {
    inner: Stream,
    counters: Arc<MetricsCounters>,
    side: Side,
}

impl<Stream> CountingStream<Stream> {
    /// The socket of a client: what is read from it goes out to the target,
    /// what is written to it came in from the target.
    pub(crate) fn on_client_side(inner: Stream, counters: Arc<MetricsCounters>) -> Self {
        Self {
            inner,
            counters,
            side: Side::Client,
        }
    }

    /// The socket of a target: what is read from it came in from the target,
    /// what is written to it goes out to the target.
    pub(crate) fn on_target_side(inner: Stream, counters: Arc<MetricsCounters>) -> Self {
        Self {
            inner,
            counters,
            side: Side::Target,
        }
    }

    fn record_read(&self, count: usize) {
        match self.side {
            Side::Client => self.counters.record_bytes_out(count),
            Side::Target => self.counters.record_bytes_in(count),
        }
    }

    fn record_written(&self, count: usize) {
        match self.side {
            Side::Client => self.counters.record_bytes_in(count),
            Side::Target => self.counters.record_bytes_out(count),
        }
    }
}

impl<Stream: AsyncRead + Unpin> AsyncRead for CountingStream<Stream> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let filled_before = buffer.filled().len();
        let poll = Pin::new(&mut self.inner).poll_read(context, buffer);
        if matches!(poll, Poll::Ready(Ok(()))) {
            self.record_read(buffer.filled().len() - filled_before);
        }
        poll
    }
}

impl<Stream: AsyncWrite + Unpin> AsyncWrite for CountingStream<Stream> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<io::Result<usize>> {
        let poll = Pin::new(&mut self.inner).poll_write(context, bytes);
        if let Poll::Ready(Ok(written)) = poll {
            self.record_written(written);
        }
        poll
    }

    fn poll_flush(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_flush(context)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_shutdown(context)
    }
}

#[cfg(test)]
mod tests {
    use tokio::io::{AsyncReadExt, AsyncWriteExt, duplex};

    use crate::clock::ManualClock;

    use super::*;

    const PAYLOAD: &[u8] = b"hello tunnel";

    fn counters() -> Arc<MetricsCounters> {
        Arc::new(MetricsCounters::new(Arc::new(ManualClock::default())))
    }

    #[tokio::test]
    async fn should_count_bytes_out_when_reading_from_the_client() {
        let counters = counters();
        let (mut client, server) = duplex(64);
        let mut counted = CountingStream::on_client_side(server, Arc::clone(&counters));
        client.write_all(PAYLOAD).await.unwrap();

        let mut buffer = [0_u8; 64];
        let count = counted.read(&mut buffer).await.unwrap();

        assert_eq!(
            (count, counters.snapshot().bytes_out),
            (PAYLOAD.len(), PAYLOAD.len() as u64)
        );
    }

    #[tokio::test]
    async fn should_count_bytes_in_when_writing_to_the_client() {
        let counters = counters();
        let (_client, server) = duplex(64);
        let mut counted = CountingStream::on_client_side(server, Arc::clone(&counters));

        counted.write_all(PAYLOAD).await.unwrap();

        assert_eq!(counters.snapshot().bytes_in, PAYLOAD.len() as u64);
    }

    #[tokio::test]
    async fn should_count_bytes_in_when_reading_from_the_target() {
        let counters = counters();
        let (mut target, server) = duplex(64);
        let mut counted = CountingStream::on_target_side(server, Arc::clone(&counters));
        target.write_all(PAYLOAD).await.unwrap();

        let mut buffer = [0_u8; 64];
        let count = counted.read(&mut buffer).await.unwrap();

        assert_eq!(
            (count, counters.snapshot().bytes_in),
            (PAYLOAD.len(), PAYLOAD.len() as u64)
        );
    }

    #[tokio::test]
    async fn should_count_bytes_out_when_writing_to_the_target() {
        let counters = counters();
        let (_target, server) = duplex(64);
        let mut counted = CountingStream::on_target_side(server, Arc::clone(&counters));

        counted.write_all(PAYLOAD).await.unwrap();

        assert_eq!(counters.snapshot().bytes_out, PAYLOAD.len() as u64);
    }

    #[tokio::test]
    async fn should_not_count_anything_when_no_byte_moves() {
        let counters = counters();
        let (_client, server) = duplex(64);
        let _counted = CountingStream::on_client_side(server, Arc::clone(&counters));

        assert_eq!(
            counters.snapshot().bytes_in + counters.snapshot().bytes_out,
            0
        );
    }

    #[tokio::test]
    async fn should_pass_the_bytes_through_unchanged_when_counting() {
        let counters = counters();
        let (mut client, server) = duplex(64);
        let mut counted = CountingStream::on_client_side(server, counters);

        counted.write_all(PAYLOAD).await.unwrap();
        let mut received = vec![0_u8; PAYLOAD.len()];
        client.read_exact(&mut received).await.unwrap();

        assert_eq!(received, PAYLOAD);
    }
}
