use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

use crate::runtime::metrics::MetricsCounters;

/// The client side of a forwarded connection, counting what passes through:
/// what is read from the client goes out to the target, what is written to
/// the client came in from the target.
pub(crate) struct CountingStream<Stream> {
    inner: Stream,
    counters: Arc<MetricsCounters>,
}

impl<Stream> CountingStream<Stream> {
    pub(crate) fn new(inner: Stream, counters: Arc<MetricsCounters>) -> Self {
        Self { inner, counters }
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
            self.counters
                .record_bytes_out(buffer.filled().len() - filled_before);
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
            self.counters.record_bytes_in(written);
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
        let mut counted = CountingStream::new(server, Arc::clone(&counters));
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
        let mut counted = CountingStream::new(server, Arc::clone(&counters));

        counted.write_all(PAYLOAD).await.unwrap();

        assert_eq!(counters.snapshot().bytes_in, PAYLOAD.len() as u64);
    }

    #[tokio::test]
    async fn should_not_count_anything_when_no_byte_moves() {
        let counters = counters();
        let (_client, server) = duplex(64);
        let _counted = CountingStream::new(server, Arc::clone(&counters));

        assert_eq!(
            counters.snapshot().bytes_in + counters.snapshot().bytes_out,
            0
        );
    }

    #[tokio::test]
    async fn should_pass_the_bytes_through_unchanged_when_counting() {
        let counters = counters();
        let (mut client, server) = duplex(64);
        let mut counted = CountingStream::new(server, counters);

        counted.write_all(PAYLOAD).await.unwrap();
        let mut received = vec![0_u8; PAYLOAD.len()];
        client.read_exact(&mut received).await.unwrap();

        assert_eq!(received, PAYLOAD);
    }
}
