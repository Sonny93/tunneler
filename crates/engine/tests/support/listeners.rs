use std::sync::{Arc, Mutex};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinHandle;

/// A TCP server that sends back every byte it receives.
pub struct EchoServer {
    pub port: u16,
    task: JoinHandle<()>,
}

impl EchoServer {
    pub async fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let task = tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                tokio::spawn(echo(stream));
            }
        });
        Self { port, task }
    }
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
    pub port: u16,
    task: JoinHandle<()>,
}

impl SilentListener {
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
