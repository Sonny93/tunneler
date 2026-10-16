//! Test helper used as a `ProxyCommand`: copies stdin to a TCP connection and
//! the connection to stdout, so tests need no `nc`.
//!
//! Usage: `tcp-bridge <host> <port> [pid-file]`.

use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::process::ExitCode;
use std::thread;

const FAILURE: u8 = 1;
const BUFFER_SIZE: usize = 8192;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("tcp-bridge: {message}");
            ExitCode::from(FAILURE)
        }
    }
}

fn run() -> Result<(), String> {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let [host, port, ..] = arguments.as_slice() else {
        return Err("usage: tcp-bridge <host> <port> [pid-file]".to_owned());
    };
    if let Some(pid_file) = arguments.get(2) {
        std::fs::write(pid_file, std::process::id().to_string()).map_err(|e| e.to_string())?;
    }
    let port: u16 = port.parse().map_err(|_| format!("invalid port {port}"))?;
    let stream = TcpStream::connect((host.as_str(), port)).map_err(|e| e.to_string())?;
    let mut upstream = stream.try_clone().map_err(|e| e.to_string())?;
    thread::spawn(move || {
        let _ = io::copy(&mut io::stdin(), &mut upstream);
    });
    copy_flushing(stream, io::stdout()).map_err(|e| e.to_string())
}

fn copy_flushing(mut source: impl Read, mut sink: impl Write) -> io::Result<()> {
    let mut buffer = [0_u8; BUFFER_SIZE];
    loop {
        let count = source.read(&mut buffer)?;
        if count == 0 {
            return Ok(());
        }
        sink.write_all(&buffer[..count])?;
        sink.flush()?;
    }
}
