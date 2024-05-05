# Arkanox

Echo server and client in Rust using mio using an event loop

# Issues

- [x] Client does not receive all messages back from server.

Fix was to read more than 16 bytes on each read (Long term this needs more robust handling, probably at a higher level than the main event loop).

- [x] Server crashes when client is killed (does not happen when using `nc` for example)

If there is unread data on client's socket and the client closes the connection, the server gets "Connection reset by peer" error. This killed the thread the connection was on, which for the server right now is the main thread.
