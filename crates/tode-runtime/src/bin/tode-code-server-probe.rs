use std::env;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<_> = env::args().skip(1).collect();
    if args == ["--version"] {
        println!("probe-code-server 1.0");
        return ExitCode::SUCCESS;
    }
    let Some(index) = args.iter().position(|argument| argument == "--bind-addr") else {
        eprintln!("missing --bind-addr");
        return ExitCode::from(2);
    };
    let Some(address) = args.get(index + 1) else {
        eprintln!("missing bind address value");
        return ExitCode::from(2);
    };
    if env::var("EXTENSIONS_GALLERY").is_err() {
        eprintln!("missing EXTENSIONS_GALLERY");
        return ExitCode::from(2);
    }
    let listener = match TcpListener::bind(address) {
        Ok(listener) => listener,
        Err(error) => {
            eprintln!("bind {address}: {error}");
            return ExitCode::from(2);
        }
    };
    for connection in listener.incoming() {
        let Ok(mut connection) = connection else {
            continue;
        };
        let mut request = [0_u8; 1024];
        let _ = connection.read(&mut request);
        let _ = connection
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok");
    }
    ExitCode::SUCCESS
}
