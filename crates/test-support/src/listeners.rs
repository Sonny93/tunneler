use std::net::Ipv4Addr;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinHandle;

/// A TCP server that sends back every byte it receives.
pub struct EchoServer {
    /// The loopback port it listens on.
    pub port: u16,
    task: JoinHandle<()>,
}

impl EchoServer {
    /// Listens on a random loopback port.
    pub async fn start() -> Self {
        Self::start_on(0).await
    }

    /// Listens on the given loopback port.
    pub async fn start_on(port: u16) -> Self {
        let listener = TcpListener::bind(("127.0.0.1", port)).await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let task = tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                tokio::spawn(echo(stream));
            }
        });
        Self { port, task }
    }

    /// Stops listening; the port is free when this returns. Connections
    /// already accepted keep echoing.
    pub async fn stop(&mut self) {
        self.task.abort();
        let _ = (&mut self.task).await;
    }
}

/// First port of the pool test ports are taken from.
const TEST_PORT_POOL_START: u16 = 20_000;
/// End (exclusive) of the pool: Linux hands out ephemeral ports from 32768,
/// Windows and macOS from 49152.
const TEST_PORT_POOL_END: u16 = 32_768;
const TEST_PORT_POOL_SIZE: u32 = (TEST_PORT_POOL_END - TEST_PORT_POOL_START) as u32;
/// Spreads concurrent test binaries over the pool.
const PROCESS_OFFSET_STRIDE: u32 = 97;

static NEXT_POOL_OFFSET: AtomicU32 = AtomicU32::new(0);

fn next_pool_port() -> u16 {
    let process_offset = std::process::id().wrapping_mul(PROCESS_OFFSET_STRIDE);
    let step = NEXT_POOL_OFFSET.fetch_add(1, Ordering::Relaxed);
    let offset = process_offset.wrapping_add(step) % TEST_PORT_POOL_SIZE;
    TEST_PORT_POOL_START + u16::try_from(offset).unwrap()
}

/// A port nobody listens on right now.
pub async fn free_port() -> u16 {
    free_ports(1).await[0]
}

/// `count` distinct ports nobody listens on right now.
///
/// The ports come from a pool below every operating system's ephemeral range,
/// so the connections other tests open in parallel can never take one of them
/// between this call and the moment the caller binds it. All the listeners
/// are held while the ports are collected, so two ports of one call never
/// collide.
pub async fn free_ports(count: usize) -> Vec<u16> {
    let mut listeners = Vec::with_capacity(count);
    let mut attempts = 0;
    while listeners.len() < count {
        assert!(
            attempts < TEST_PORT_POOL_SIZE,
            "no free port left in the test pool {TEST_PORT_POOL_START}..{TEST_PORT_POOL_END}"
        );
        attempts += 1;
        if let Ok(listener) = TcpListener::bind((Ipv4Addr::LOCALHOST, next_pool_port())).await {
            listeners.push(listener);
        }
    }
    listeners
        .iter()
        .map(|listener| listener.local_addr().unwrap().port())
        .collect()
}

impl Drop for EchoServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn echo(mut stream: TcpStream) {
    let mut buffer = [0_u8; 1024];
    while let Ok(count) = stream.read(&mut buffer).await {
        if count == 0 || stream.write_all(&buffer[..count]).await.is_err() {
            return;
        }
    }
}

/// A TCP listener that accepts connections and never says a word.
pub struct SilentListener {
    /// The loopback port it listens on.
    pub port: u16,
    task: JoinHandle<()>,
}

impl SilentListener {
    /// Listens on a random loopback port.
    pub async fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let held = Arc::new(Mutex::new(Vec::new()));
        let task = tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                held.lock().unwrap().push(stream);
            }
        });
        Self { port, task }
    }
}

impl Drop for SilentListener {
    fn drop(&mut self) {
        self.task.abort();
    }
}
