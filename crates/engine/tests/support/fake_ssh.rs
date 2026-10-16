//! Test helper standing in for `ssh`: binds the listen port of every `-L`
//! forward of its command line, then sleeps until it is killed.
//!
//! Usage: `ssh -N -L <port>:<host>:<port> <destination>`.

use std::net::TcpListener;
use std::process::ExitCode;
use std::thread;

use engine::parse_ssh_command_line;

const FAILURE: u8 = 1;
const DEFAULT_BIND_ADDRESS: &str = "127.0.0.1";

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().collect();
    let invocation = parse_ssh_command_line(&arguments);
    let mut listeners = Vec::new();
    for forward in invocation.forwards {
        let address = forward
            .listen_address
            .as_deref()
            .unwrap_or(DEFAULT_BIND_ADDRESS);
        match TcpListener::bind((address, forward.listen_port)) {
            Ok(listener) => listeners.push(listener),
            Err(error) => {
                eprintln!(
                    "fake-ssh: cannot bind {address}:{}: {error}",
                    forward.listen_port
                );
                return ExitCode::from(FAILURE);
            }
        }
    }
    loop {
        thread::park();
    }
}
