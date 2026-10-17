use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use engine::{DisplayPlatform, X11Environment};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinHandle;

const FIRST_DISPLAY_NUMBER: u16 = 20;
const LAST_DISPLAY_NUMBER: u16 = 400;
const TCP_BASE_PORT: u16 = 6000;
const MIT_MAGIC_COOKIE: &[u8] = b"MIT-MAGIC-COOKIE-1";
const FAMILY_INTERNET: u16 = 0;
const LOOPBACK: [u8; 4] = [127, 0, 0, 1];
const SETUP_MAJOR_VERSION: u16 = 11;
const FIELD_PADDING_UNIT: usize = 4;

/// A machine whose X display and credentials the test chooses.
#[derive(Debug, Clone, Default)]
pub struct FakeX11Environment {
    pub display: Option<String>,
    pub xauthority: Option<PathBuf>,
}

impl X11Environment for FakeX11Environment {
    fn display(&self) -> Option<String> {
        self.display.clone()
    }

    fn xauthority_path(&self) -> Option<PathBuf> {
        self.xauthority.clone()
    }

    fn home_directory(&self) -> Option<PathBuf> {
        None
    }

    fn host_name(&self) -> String {
        "testbed".to_owned()
    }

    fn platform(&self) -> DisplayPlatform {
        DisplayPlatform::Unix
    }
}

/// A TCP listener on `6000 + N` that records what X clients send and answers
/// the first thing they say with a fixed reply.
pub struct FakeXServer {
    pub display_number: u16,
    received: Arc<Mutex<Vec<u8>>>,
    connections: Arc<AtomicUsize>,
    task: JoinHandle<()>,
}

impl FakeXServer {
    /// Listens on the first free display number.
    pub async fn start(reply: &'static [u8]) -> Self {
        let (listener, display_number) = bind_first_free_display().await;
        let received = Arc::new(Mutex::new(Vec::new()));
        let connections = Arc::new(AtomicUsize::new(0));
        let task = tokio::spawn(accept_clients(
            listener,
            reply,
            Arc::clone(&received),
            Arc::clone(&connections),
        ));
        Self {
            display_number,
            received,
            connections,
            task,
        }
    }

    /// The `DISPLAY` value that reaches this server over TCP.
    pub fn display(&self) -> String {
        format!("127.0.0.1:{}", self.display_number)
    }

    /// Everything clients sent, in arrival order.
    pub fn received(&self) -> Vec<u8> {
        self.received.lock().unwrap().clone()
    }

    pub fn connection_count(&self) -> usize {
        self.connections.load(Ordering::SeqCst)
    }

    /// Writes an `Xauthority` file holding `cookie` for this server.
    pub fn write_authority(&self, path: &Path, cookie: &[u8]) {
        let mut bytes = Vec::new();
        bytes.extend(FAMILY_INTERNET.to_be_bytes());
        for block in [
            LOOPBACK.as_slice(),
            self.display_number.to_string().as_bytes(),
            MIT_MAGIC_COOKIE,
            cookie,
        ] {
            bytes.extend(u16::try_from(block.len()).unwrap().to_be_bytes());
            bytes.extend(block);
        }
        std::fs::write(path, bytes).unwrap();
    }
}

impl Drop for FakeXServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn bind_first_free_display() -> (TcpListener, u16) {
    for display_number in FIRST_DISPLAY_NUMBER..=LAST_DISPLAY_NUMBER {
        let port = TCP_BASE_PORT + display_number;
        if let Ok(listener) = TcpListener::bind(("127.0.0.1", port)).await {
            return (listener, display_number);
        }
    }
    panic!("no free X display number between {FIRST_DISPLAY_NUMBER} and {LAST_DISPLAY_NUMBER}");
}

async fn accept_clients(
    listener: TcpListener,
    reply: &'static [u8],
    received: Arc<Mutex<Vec<u8>>>,
    connections: Arc<AtomicUsize>,
) {
    while let Ok((stream, _)) = listener.accept().await {
        connections.fetch_add(1, Ordering::SeqCst);
        tokio::spawn(serve_client(stream, reply, Arc::clone(&received)));
    }
}

async fn serve_client(mut stream: TcpStream, reply: &'static [u8], received: Arc<Mutex<Vec<u8>>>) {
    let mut buffer = [0_u8; 1024];
    let mut has_replied = false;
    while let Ok(count) = stream.read(&mut buffer).await {
        if count == 0 {
            return;
        }
        received.lock().unwrap().extend(&buffer[..count]);
        if !has_replied {
            has_replied = true;
            if stream.write_all(reply).await.is_err() {
                return;
            }
        }
    }
}

/// The first bytes an X client sends: byte order, version, then the
/// authorization protocol name and data, each padded to four bytes.
pub fn x_client_setup(name: &[u8], data: &[u8]) -> Vec<u8> {
    let mut setup = vec![0x42, 0];
    setup.extend(SETUP_MAJOR_VERSION.to_be_bytes());
    setup.extend(0_u16.to_be_bytes());
    setup.extend(u16::try_from(name.len()).unwrap().to_be_bytes());
    setup.extend(u16::try_from(data.len()).unwrap().to_be_bytes());
    setup.extend([0, 0]);
    for field in [name, data] {
        setup.extend(field);
        setup.resize(setup.len() + padding_of(field.len()), 0);
    }
    setup
}

/// The setup an X client sends when it presents the session cookie.
pub fn x_client_setup_with_cookie(cookie: &[u8]) -> Vec<u8> {
    x_client_setup(MIT_MAGIC_COOKIE, cookie)
}

/// The setup the local X server should receive for `cookie`.
pub fn expected_server_setup(cookie: &[u8]) -> Vec<u8> {
    x_client_setup(MIT_MAGIC_COOKIE, cookie)
}

fn padding_of(length: usize) -> usize {
    length.next_multiple_of(FIELD_PADDING_UNIT) - length
}

/// Decodes the hexadecimal cookie the server recorded.
pub fn decode_hex(text: &str) -> Vec<u8> {
    (0..text.len())
        .step_by(2)
        .map(|start| u8::from_str_radix(&text[start..start + 2], 16).unwrap())
        .collect()
}
