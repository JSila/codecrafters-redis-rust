use std::io::{Read, Write};
use std::net::TcpListener;

fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    // Uncomment this block to pass the first stage
    let listener = TcpListener::bind("127.0.0.1:6379").unwrap();

    let mut buf = [0; 512];

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => loop {
                let bytes_read = stream.read(&mut buf).unwrap();
                if bytes_read == 0 {
                    break;
                }
                stream.write_all(b"+PONG\r\n").unwrap();
            },
            Err(e) => {
                eprintln!("error: {}", e);
            }
        }
    }
}
