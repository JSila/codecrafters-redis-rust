use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

fn main() {
    let listener = TcpListener::bind("127.0.0.1:6379").unwrap();
    let mut threads = vec![];

    for _ in 0..4 {
        let listener = listener.try_clone().unwrap();
        threads.push(thread::spawn(move || {
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
        }));
    }

    for t in threads {
        t.join().unwrap();
    }
}
