use mio::{
    event::Event,
    net::{TcpListener, TcpStream},
    Events, Interest, Poll, Registry, Token,
};
use slab::Slab;
use std::{
    collections::VecDeque,
    io::{self, ErrorKind, Read, Write},
};

const MAX_CONNECTIONS: usize = 1000;
const SERVER: Token = Token(1024);

fn main() -> anyhow::Result<()> {
    let mut poll = Poll::new()?;

    let mut events = Events::with_capacity(1024);

    let mut listener = TcpListener::bind("127.0.0.1:9000".parse()?)?;

    poll.registry()
        .register(&mut listener, SERVER, Interest::READABLE)?;

    let mut connections: Slab<TcpStream> = Slab::with_capacity(MAX_CONNECTIONS);
    let mut send_queue: VecDeque<String> = VecDeque::new();

    // TODO: Consider moving this in a separate thread
    loop {
        // Block until we receive a readiness event
        poll.poll(&mut events, None)?;

        for event in events.iter() {
            match event.token() {
                SERVER => loop {
                    let (mut connection, address) = match listener.accept() {
                        Ok((connection, address)) => (connection, address),
                        Err(e) if e.kind() == ErrorKind::WouldBlock => {
                            break;
                        }
                        Err(e) => return Err(anyhow::Error::new(e)),
                    };

                    println!("Accepted connection from {}", address);

                    if connections.len() == connections.capacity() {
                        // Shutdown the server, we have reached maximum
                        // number of connections we can accept
                        return Ok(());
                    }

                    let next = connections.vacant_key();

                    poll.registry().register(
                        &mut connection,
                        Token(next),
                        Interest::READABLE.add(Interest::WRITABLE),
                    )?;

                    connections.insert(connection);
                },
                token => {
                    let done = if let Some(connection) = connections.get_mut(token.0) {
                        handle(poll.registry(), event, connection, &mut send_queue)?
                    } else {
                        // If there is no connection
                        false
                    };
                    if done {
                        if let Some(mut connection) = connections.try_remove(token.0) {
                            poll.registry().deregister(&mut connection)?;
                        }
                    }
                }
            }
        }
    }
}

fn handle(
    registry: &Registry,
    event: &Event,
    connection: &mut TcpStream,
    send_queue: &mut VecDeque<String>,
) -> io::Result<bool> {
    if event.is_readable() {
        // TODO: Consider using `BytesMut` here
        let mut received = vec![0; 4096];
        let mut connection_closed = false;
        let mut bytes_read = 0;
        loop {
            match connection.read(&mut received[bytes_read..]) {
                Ok(0) => {
                    println!("0 byte read");
                    connection_closed = true;
                    break;
                }
                Ok(n) => {
                    println!("Read {} bytes", n);
                    bytes_read += n;
                    if bytes_read == received.len() {
                        received.resize(received.len() + 1024, 0);
                    }
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => break,
                Err(e) if e.kind() == ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            }
        }

        if bytes_read != 0 {
            let received = &received[..bytes_read];
            if let Ok(str_buf) = std::str::from_utf8(&received) {
                println!("Received: {}", str_buf);
                if let Ok(()) = connection.write_all(str_buf.as_bytes()) {
                    return Ok(false);
                }
                // Enqueue the write if we werent able to write it here
                send_queue.push_back(str_buf.to_string());
            }
        }

        if connection_closed {
            println!("Connection closed");
            return Ok(true);
        }
    }

    if event.is_writable() {
        if let Some(msg) = send_queue.pop_front() {
            match connection.write(msg.as_bytes()) {
                Ok(n) if n < msg.as_bytes().len() => return Err(ErrorKind::WriteZero.into()),
                Ok(_) => {
                    println!("send {} on connection", msg);
                    // re-register for both read and write events
                    registry.reregister(
                        connection,
                        event.token(),
                        Interest::READABLE.add(Interest::WRITABLE),
                    )?;
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                    // put the message back for resending
                    send_queue.push_front(msg)
                }
                Err(e) if e.kind() == ErrorKind::Interrupted => {
                    // retry
                    return handle(registry, event, connection, send_queue);
                }
                Err(e) => return Err(e),
            }
        }
    }

    Ok(false)
}
