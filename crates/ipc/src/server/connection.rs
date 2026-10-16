use std::sync::Arc;
use std::time::Duration;

use interprocess::local_socket::tokio::Stream;
use tokio::io::{AsyncWriteExt, BufReader, ReadHalf, WriteHalf, split};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::protocol::{
    Hello, HelloResult, IpcError, LineRead, MAX_LINE_BYTES, PROTOCOL_VERSION, Request,
    RequestEnvelope, ResponsePayload, read_line,
};
use crate::server::handler::{ConnectionId, ConnectionInfo, PeerInfo, RequestHandler};
use crate::server::outbound::{Frame, OutboundQueue};
use crate::server::registry::Registry;

/// How long a client may take to say `hello`.
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);

pub(crate) struct ConnectionSettings {
    pub(crate) daemon_version: String,
    pub(crate) outbound_queue_capacity: usize,
}

pub(crate) struct Connection {
    pub(crate) id: ConnectionId,
    pub(crate) peer: PeerInfo,
    pub(crate) registry: Arc<Registry>,
    pub(crate) handler: Arc<dyn RequestHandler>,
    pub(crate) settings: Arc<ConnectionSettings>,
    pub(crate) cancellation: CancellationToken,
}

type Reader = BufReader<ReadHalf<Stream>>;

enum HandshakeOutcome {
    Accepted(Arc<ConnectionInfo>),
    Refused,
}

impl Connection {
    pub(crate) async fn run(self, stream: Stream) {
        let (read_half, write_half) = split(stream);
        let mut reader = BufReader::new(read_half);
        let (outbound, receiver) = OutboundQueue::new(
            self.settings.outbound_queue_capacity,
            self.cancellation.clone(),
        );
        let writer = tokio::spawn(write_lines(write_half, receiver, self.cancellation.clone()));

        self.serve(&mut reader, &outbound).await;

        self.registry.unregister(self.id);
        if self.cancellation.is_cancelled() {
            writer.abort();
        } else {
            outbound.close();
            let _ = writer.await;
        }
        self.cancellation.cancel();
    }

    async fn serve(&self, reader: &mut Reader, outbound: &OutboundQueue) {
        let handshake = tokio::select! {
            () = self.cancellation.cancelled() => return,
            outcome = tokio::time::timeout(
                HANDSHAKE_TIMEOUT,
                self.perform_handshake(reader, outbound),
            ) => outcome,
        };
        match handshake {
            Ok(HandshakeOutcome::Accepted(info)) => {
                self.serve_requests(reader, outbound, info).await;
            }
            Ok(HandshakeOutcome::Refused) => {}
            Err(_) => tracing::debug!(connection = %self.id, "handshake timed out"),
        }
    }

    async fn perform_handshake(
        &self,
        reader: &mut Reader,
        outbound: &OutboundQueue,
    ) -> HandshakeOutcome {
        loop {
            let line = match read_line(reader, MAX_LINE_BYTES).await {
                Ok(LineRead::Line(line)) => line,
                Ok(LineRead::TooLong) => {
                    outbound.respond(None, &Err(line_too_long()));
                    return HandshakeOutcome::Refused;
                }
                Ok(LineRead::EndOfStream) | Err(_) => return HandshakeOutcome::Refused,
            };
            match RequestEnvelope::parse(&line) {
                Err(rejection) => outbound.respond(rejection.id, &Err(rejection.error)),
                Ok(RequestEnvelope {
                    id,
                    request: Request::Hello(hello),
                }) => return self.accept_hello(id, &hello, outbound),
                Ok(envelope) => {
                    outbound.respond(Some(envelope.id), &Err(IpcError::handshake_required()));
                    return HandshakeOutcome::Refused;
                }
            }
        }
    }

    fn accept_hello(&self, id: u64, hello: &Hello, outbound: &OutboundQueue) -> HandshakeOutcome {
        if hello.protocol_version != PROTOCOL_VERSION {
            let error = IpcError::protocol_mismatch(PROTOCOL_VERSION, hello.protocol_version);
            outbound.respond(Some(id), &Err(error));
            return HandshakeOutcome::Refused;
        }
        let info = Arc::new(ConnectionInfo {
            id: self.id,
            client_kind: hello.client_kind,
            client_version: hello.client_version.clone(),
            subscribes_to_events: hello.subscribe_events,
            can_prompt: hello.can_prompt,
            peer: self.peer,
        });
        let result = HelloResult {
            protocol_version: PROTOCOL_VERSION,
            daemon_version: self.settings.daemon_version.clone(),
        };
        outbound.respond(Some(id), &Ok(ResponsePayload::Hello(result)));
        self.registry
            .register(info.as_ref().clone(), outbound.clone());
        HandshakeOutcome::Accepted(info)
    }

    async fn serve_requests(
        &self,
        reader: &mut Reader,
        outbound: &OutboundQueue,
        info: Arc<ConnectionInfo>,
    ) {
        tracing::debug!(connection = %self.id, kind = ?info.client_kind, "client connected");
        loop {
            let read = tokio::select! {
                () = self.cancellation.cancelled() => return,
                read = read_line(reader, MAX_LINE_BYTES) => read,
            };
            match read {
                Ok(LineRead::Line(line)) => self.dispatch(&line, outbound, &info),
                Ok(LineRead::TooLong) => {
                    outbound.respond(None, &Err(line_too_long()));
                    return;
                }
                Ok(LineRead::EndOfStream) => return,
                Err(error) => {
                    tracing::debug!(connection = %self.id, %error, "read failed");
                    return;
                }
            }
        }
    }

    fn dispatch(&self, line: &[u8], outbound: &OutboundQueue, info: &Arc<ConnectionInfo>) {
        let envelope = match RequestEnvelope::parse(line) {
            Ok(envelope) => envelope,
            Err(rejection) => {
                outbound.respond(rejection.id, &Err(rejection.error));
                return;
            }
        };
        if matches!(envelope.request, Request::Hello(_)) {
            let error = IpcError::invalid_request("the handshake is already done");
            outbound.respond(Some(envelope.id), &Err(error));
            return;
        }
        let handler = Arc::clone(&self.handler);
        let info = Arc::clone(info);
        let outbound = outbound.clone();
        let cancellation = self.cancellation.clone();
        tokio::spawn(async move {
            let outcome = tokio::select! {
                () = cancellation.cancelled() => return,
                outcome = handler.handle(&info, envelope.request) => outcome,
            };
            outbound.respond(Some(envelope.id), &outcome);
        });
    }
}

fn line_too_long() -> IpcError {
    IpcError::invalid_request(format!("a line cannot exceed {MAX_LINE_BYTES} bytes"))
}

async fn write_lines(
    mut writer: WriteHalf<Stream>,
    mut queue: mpsc::Receiver<Frame>,
    cancellation: CancellationToken,
) {
    while let Some(item) = queue.recv().await {
        let Frame::Line(line) = item else {
            break;
        };
        let written = async {
            writer.write_all(line.as_bytes()).await?;
            writer.flush().await
        }
        .await;
        if let Err(error) = written {
            tracing::debug!(%error, "write failed");
            cancellation.cancel();
            return;
        }
    }
    if let Err(error) = writer.shutdown().await {
        tracing::debug!(%error, "shutdown failed");
    }
}
