use std::{
    io::{Read, Write},
    net::TcpStream,
};

fn main() -> anyhow::Result<()> {
    let mut connection = TcpStream::connect("127.0.0.1:9000")?;

    connection.write_all("hello world!".as_bytes())?;
    connection.flush()?;
    println!("Sent message to server");
    let mut buf = vec![0; 12];
    connection.read_exact(&mut buf)?;
    println!("Received {} from server", std::str::from_utf8(&buf)?);
    Ok(())
}
