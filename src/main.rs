use std::collections::HashMap;
use std::str::Split;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

type Db = Arc<Mutex<HashMap<String, String>>>;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:6379").await?;

    let db: Db = Default::default();

    loop {
        let db = db.clone();

        match listener.accept().await {
            Ok((stream, _)) => {
                tokio::spawn(async move {
                    handle_connection(stream, db).await.unwrap();
                });
            }
            Err(e) => {
                eprintln!("error: {}", e);
            }
        }
    }
}

async fn handle_connection(mut stream: TcpStream, db: Db) -> anyhow::Result<()> {
    let mut buf = [0; 512];

    loop {
        let bytes_read = stream.read(&mut buf).await?;
        if bytes_read == 0 {
            break;
        }

        let output = handle_resp(&buf[..bytes_read], &db)?;

        stream.write(&output.as_bytes()).await?;
    }

    Ok(())
}

fn handle_resp(buf: &[u8], db: &Db) -> anyhow::Result<String> {
    dbg!(std::str::from_utf8(buf).unwrap());

    let request_value = Value::parse(buf)?;

    dbg!(&request_value);

    let response_value = if let Value::Array(a) = request_value {
        if let Some(Value::BulkString(command)) = a.get(0) {
            let command = command.to_ascii_lowercase();
            match command.as_str() {
                "ping" => Value::SimpleString("PONG".to_string()),
                "echo" => a
                    .get(1..)
                    .map(|args| args.first().unwrap().clone())
                    .unwrap_or(Value::Error("no args to echo".to_string())),
                "get" => {
                    if let Some(Value::BulkString(k)) = a.get(1) {
                        let db = db.lock().unwrap();
                        db.get(k)
                            .map(|res| Value::BulkString(res.to_string()))
                            .unwrap_or(Value::Nil)
                    } else {
                        Value::Error("no key for get".to_string())
                    }
                }
                "set" => {
                    if let (Some(Value::BulkString(key)), Some(Value::BulkString(value))) =
                        (a.get(1), a.get(2))
                    {
                        let mut db = db.lock().unwrap();
                        db.insert(key.into(), value.into());
                        Value::SimpleString("OK".to_string())
                    } else {
                        Value::Error("no key and value for set".into())
                    }
                }
                _ => Value::Error("command not supported".to_string()),
            }
        } else {
            Value::Error("command is not bulk string".to_string())
        }
    } else {
        Value::Error("RESP transmitted is not array".to_string())
    };

    dbg!(&response_value);
    dbg!(&response_value.to_resp());

    Ok(response_value.to_resp())
}

#[derive(Debug, Clone)]
enum Value {
    Array(Vec<Value>),
    BulkString(String),
    Error(String),
    Integer(String),
    SimpleString(String),
    Nil,
}

impl Value {
    pub fn parse(buf: &[u8]) -> anyhow::Result<Value> {
        let resp = std::str::from_utf8(buf)?;
        let mut split = resp.split("\r\n");
        parse(&mut split)
    }

    fn to_resp(&self) -> String {
        match self {
            Value::Array(v) => format!(
                "*{}\r\n{}",
                v.len(),
                v.iter().map(|u| u.to_resp()).collect::<String>()
            ),
            Value::BulkString(s) => format!("${}\r\n{}\r\n", s.len(), s),
            Value::Error(s) => format!("-{}\r\n", s),
            Value::Integer(s) => format!(":{}\r\n", s),
            Value::SimpleString(s) => format!("+{}\r\n", s),
            Value::Nil => String::from("*-1\r\n"),
        }
    }
}

fn parse(split: &mut Split<&str>) -> anyhow::Result<Value> {
    let resp = split.next().unwrap();

    match resp.get(0..1) {
        Some("*") => {
            let mut array = vec![];
            let mut array_size = resp[1..].parse::<i64>().unwrap();

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
