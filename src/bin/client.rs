use std::io::{ErrorKind, Read, Write};

use mio::{net::TcpStream, Events, Interest, Poll, Token};

const CLIENT: Token = Token(0);
const MAX_MSGS: usize = 10;

fn main() -> anyhow::Result<()> {
    let mut connection = TcpStream::connect("127.0.0.1:9000".parse()?)?;

    let mut poll = Poll::new()?;
    let mut events = Events::with_capacity(1024);

    poll.registry().register(
        &mut connection,
        CLIENT,
        Interest::WRITABLE.add(Interest::READABLE),
    )?;

    let mut write_count = 0;
    let mut read_count = 0;

    // main event loop
    loop {
        poll.poll(&mut events, None)?;

        for event in events.iter() {
            if event.token().0 != CLIENT.0 {
                panic!("Got an event for unregistered token!");
            }

            if event.is_writable() {
                match connection.peer_addr() {
                    Ok(_) => {
                        if write_count < MAX_MSGS {
                            let msg = format!("[{}] hello world!", write_count);
                            // TODO: We dont handle the case where we werent able to write the full message
                            match connection.write(msg.as_bytes()) {
                                Ok(_n) => {
                                    write_count += 1;
                                }
                                Err(e) => {
                                    println!("Error {} when writing to socket", e);
                                }
                            }
                        }
                    }
                    Err(ref e) if e.raw_os_error() == Some(libc::EINPROGRESS) => continue,
                    Err(e) if e.kind() == ErrorKind::NotConnected => continue,
                    Err(e) if e.kind() == ErrorKind::WouldBlock => continue,
                    Err(_) => {
                        panic!("Could not connect to server");
                    }
                }
            }

            if event.is_readable() {
                let mut buf = vec![0; 16];
                let mut connection_closed = false;
                match connection.read(&mut buf) {
                    Ok(0) => {
                        println!("0 byte read");
                        connection_closed = true;
                    }
                    Ok(_) => {
                        println!("Received {} from server", String::from_utf8(buf)?);
                        read_count += 1;
                        if read_count == MAX_MSGS {
                            return Ok(());
                        }
                    }
                    Err(e) if e.kind() == ErrorKind::WouldBlock => continue,
                    Err(e) if e.kind() == ErrorKind::Interrupted => continue,
                    Err(e) => panic!("Error {} when reading from socket", e),
                }

                if connection_closed {
                    return Ok(());
                }
            }
        }
    }
}
