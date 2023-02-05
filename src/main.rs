use std::str::Split;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:6379").await?;

    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                tokio::spawn(async move {
                    handle_connection(stream).await.unwrap();
                });
            }
            Err(e) => {
                eprintln!("error: {}", e);
            }
        }
    }
}

async fn handle_connection(mut stream: TcpStream) -> anyhow::Result<()> {
    let mut buf = [0; 512];

    loop {
        let bytes_read = stream.read(&mut buf).await?;
        if bytes_read == 0 {
            break;
        }

        let output = handle_resp(&buf[..bytes_read])?;

        stream.write(&output.as_bytes()).await?;
    }

    Ok(())
}

fn handle_resp(buf: &[u8]) -> anyhow::Result<String> {
    let request_value = Value::parse(buf)?;

    // dbg!(&request_value);

    let response_value = match request_value {
        Value::Array(a) => {
            let command = a.get(0);
            match command {
                Some(Value::BulkString(s)) if s.eq("PING") => {
                    Value::SimpleString("PONG".to_string())
                }
                Some(Value::BulkString(s)) if s.eq("ECHO") => {
                    let args = a.get(1..);
                    match args {
                        None => Value::Error("".to_string()),
                        Some(args) => Value::Array(args.to_vec()),
                    }
                }
                None => Value::Error("".to_string()),
                _ => Value::Error("".to_string()),
            }
        }
        Value::BulkString(_) => Value::Error("".to_string()),
        Value::Error(_) => Value::Error("".to_string()),
        Value::Integer(_) => Value::Error("".to_string()),
        Value::SimpleString(_) => Value::Error("".to_string()),
    };

    // dbg!(&response_value);
    // dbg!(&response_value.to_resp());

    Ok(response_value.to_resp())
}

#[derive(Debug, Clone)]
enum Value {
    Array(Vec<Value>),
    BulkString(String),
    Error(String),
    Integer(String),
    SimpleString(String),
}

impl Value {
    pub fn parse(buf: &[u8]) -> anyhow::Result<Value> {
        let resp = std::str::from_utf8(buf)?;
        let mut split = resp.split("\r\n");
        parse(&mut split)
    }

    fn to_resp(&self) -> String {
        match self {
            Value::Array(v) => {
                let mut s = format!("*{}\r\n", v.len());
                for u in v {
                    s.push_str(&u.to_resp());
                }
                s
            }
            Value::BulkString(s) => {
                format!("${}\r\n{}\r\n", s.len(), s)
            }
            Value::Error(s) => {
                format!("-{}\r\n", s)
            }
            Value::Integer(s) => {
                format!(":{}\r\n", s)
            }
            Value::SimpleString(s) => {
                format!("+{}\r\n", s)
            }
        }
    }
}

fn parse(split: &mut Split<&str>) -> anyhow::Result<Value> {
    let resp = split.next().unwrap();

    match resp.get(0..1) {
        Some("*") => {
            let mut array_size = resp[1..].parse::<i64>().unwrap();

            let mut array = vec![];

            while array_size > 0 {
                array.push(parse(split)?);
                array_size -= 1;
            }

            Ok(Value::Array(array))
        }
        Some("$") => {
            let string = split.next().unwrap();
            Ok(Value::BulkString(string.to_string()))
        }
        Some("+") => Ok(Value::SimpleString(resp[1..].to_string())),
        Some("-") => Ok(Value::Error(resp[1..].to_string())),
        Some(":") => Ok(Value::Integer(resp[1..].to_string())),
        Some(_) => Err(anyhow::anyhow!("Some(_)")),
        None => Err(anyhow::anyhow!("None")),
    }
}
