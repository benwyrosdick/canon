use std::{
    collections::HashMap,
    io::{self, Write},
    net::{TcpListener, TcpStream, ToSocketAddrs},
    sync::{Arc, Mutex, mpsc},
    thread,
};

use super::wire::{
    self, ERR_BUSY, ERR_EXISTS, ERR_FULL, ERR_MISSING, ERROR, Hello, JOINED, PEER_JOINED,
    PEER_LEFT, PING, PONG, RoomCode,
};

const MAX_ROOMS: usize = 64;

struct Slot {
    tx: mpsc::Sender<Vec<u8>>,
}

#[derive(Default)]
struct Room {
    host: Option<Slot>,
    guest: Option<Slot>,
}

struct Hub {
    rooms: HashMap<RoomCode, Room>,
}

impl Hub {
    fn other(&self, code: RoomCode, as_host: bool) -> Option<mpsc::Sender<Vec<u8>>> {
        let room = self.rooms.get(&code)?;
        let slot = if as_host { &room.guest } else { &room.host };
        slot.as_ref().map(|slot| slot.tx.clone())
    }
}

pub fn run(addr: impl ToSocketAddrs) -> io::Result<()> {
    let listener = TcpListener::bind(addr)?;
    println!("canon-relay bound {}", listener.local_addr()?);
    let _ = io::stdout().flush();
    serve(listener)
}

pub fn serve(listener: TcpListener) -> io::Result<()> {
    listener.set_nonblocking(false)?;
    let hub = Arc::new(Mutex::new(Hub {
        rooms: HashMap::new(),
    }));
    loop {
        let (stream, _) = listener.accept()?;
        let hub = hub.clone();
        thread::spawn(move || {
            if let Err(error) = handle(stream, hub)
                && error.kind() != io::ErrorKind::UnexpectedEof
            {
                eprintln!("relay client: {error}");
            }
        });
    }
}

fn handle(stream: TcpStream, hub: Arc<Mutex<Hub>>) -> io::Result<()> {
    stream.set_nodelay(true)?;
    let mut reader = stream.try_clone()?;
    let writer = stream;
    let hello = Hello::decode(&wire::read_frame(&mut reader)?).ok_or_else(|| {
        let _ = reply(&writer, &[ERROR, wire::ERR_BAD]);
        io::Error::new(io::ErrorKind::InvalidData, "bad hello")
    })?;
    let (code, as_host) = match hello {
        Hello::Host(code) => (code, true),
        Hello::Guest(code) => (code, false),
    };
    let (rx, echo) = {
        let mut hub = hub.lock().expect("relay hub");
        match register(&mut hub, code, as_host) {
            Ok(pair) => pair,
            Err(code) => {
                let _ = reply(&writer, &[ERROR, code]);
                return Ok(());
            }
        }
    };
    reply(&writer, &[JOINED, u8::from(!as_host)])?;
    if !as_host && let Some(host) = hub.lock().expect("relay hub").other(code, false) {
        let _ = host.send(vec![PEER_JOINED]);
    }
    let mut writer_clone = writer.try_clone()?;
    let write_thread = thread::spawn(move || {
        while let Ok(frame) = rx.recv() {
            if wire::write_frame(&mut writer_clone, &frame).is_err() {
                break;
            }
        }
    });
    let result = (|| -> io::Result<()> {
        loop {
            let frame = wire::read_frame(&mut reader)?;
            if frame.first() == Some(&PING) {
                let _ = echo.send(vec![PONG]);
            }
            if let Some(peer) = hub.lock().expect("relay hub").other(code, as_host)
                && peer.send(frame).is_err()
            {
                break;
            }
        }
        Ok(())
    })();
    let leftover = {
        let mut hub = hub.lock().expect("relay hub");
        leave(&mut hub, code, as_host)
    };
    if let Some(peer) = leftover {
        let _ = peer.send(vec![PEER_LEFT]);
    }
    drop(write_thread);
    result
}

fn reply(mut stream: &TcpStream, payload: &[u8]) -> io::Result<()> {
    wire::write_frame(&mut stream, payload)
}

type Mail = (mpsc::Receiver<Vec<u8>>, mpsc::Sender<Vec<u8>>);

fn register(hub: &mut Hub, code: RoomCode, as_host: bool) -> Result<Mail, u8> {
    let (tx, rx) = mpsc::channel();
    let echo = tx.clone();
    if as_host {
        if hub.rooms.contains_key(&code) {
            return Err(ERR_EXISTS);
        }
        if hub.rooms.len() >= MAX_ROOMS {
            return Err(ERR_BUSY);
        }
        hub.rooms.insert(
            code,
            Room {
                host: Some(Slot { tx }),
                guest: None,
            },
        );
        Ok((rx, echo))
    } else {
        let Some(room) = hub.rooms.get_mut(&code) else {
            return Err(ERR_MISSING);
        };
        if room.guest.is_some() {
            return Err(ERR_FULL);
        }
        room.guest = Some(Slot { tx });
        Ok((rx, echo))
    }
}

fn leave(hub: &mut Hub, code: RoomCode, as_host: bool) -> Option<mpsc::Sender<Vec<u8>>> {
    let other = hub.other(code, as_host);
    let Some(room) = hub.rooms.get_mut(&code) else {
        return other;
    };
    if as_host {
        room.host = None;
    } else {
        room.guest = None;
    }
    if room.host.is_none() && room.guest.is_none() {
        hub.rooms.remove(&code);
    }
    other
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::wire::{read_frame, write_frame};
    use std::net::Shutdown;
    use std::time::Duration;

    fn start() -> std::net::SocketAddr {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        thread::spawn(move || serve(listener).ok());
        addr
    }

    fn join(addr: std::net::SocketAddr, hello: Hello) -> TcpStream {
        let mut stream = TcpStream::connect(addr).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        write_frame(&mut stream, &hello.encode()).unwrap();
        stream
    }

    #[test]
    fn host_and_guest_exchange_and_third_is_rejected() {
        let addr = start();
        let code = RoomCode(*b"7K2Q");
        let mut host = join(addr, Hello::Host(code));
        assert_eq!(read_frame(&mut host).unwrap(), vec![JOINED, 0]);
        let mut guest = join(addr, Hello::Guest(code));
        assert_eq!(read_frame(&mut guest).unwrap(), vec![JOINED, 1]);
        assert_eq!(read_frame(&mut host).unwrap(), vec![PEER_JOINED]);
        write_frame(&mut host, b"aim").unwrap();
        assert_eq!(read_frame(&mut guest).unwrap(), b"aim");
        write_frame(&mut guest, b"fire").unwrap();
        assert_eq!(read_frame(&mut host).unwrap(), b"fire");
        let mut third = join(addr, Hello::Guest(code));
        let reply = read_frame(&mut third).unwrap();
        assert_eq!(reply[0], ERROR);
        assert_eq!(reply[1], ERR_FULL);
        host.shutdown(Shutdown::Both).ok();
        assert_eq!(read_frame(&mut guest).unwrap(), vec![PEER_LEFT]);
    }

    #[test]
    fn ping_is_answered_while_waiting_and_forwarded_to_peer() {
        let addr = start();
        let code = RoomCode(*b"P2NG");
        let mut host = join(addr, Hello::Host(code));
        assert_eq!(read_frame(&mut host).unwrap(), vec![JOINED, 0]);
        write_frame(&mut host, &[PING]).unwrap();
        assert_eq!(read_frame(&mut host).unwrap(), vec![PONG]);
        let mut guest = join(addr, Hello::Guest(code));
        assert_eq!(read_frame(&mut guest).unwrap(), vec![JOINED, 1]);
        assert_eq!(read_frame(&mut host).unwrap(), vec![PEER_JOINED]);
        write_frame(&mut host, &[PING]).unwrap();
        assert_eq!(read_frame(&mut host).unwrap(), vec![PONG]);
        assert_eq!(read_frame(&mut guest).unwrap(), vec![PING]);
    }

    #[test]
    fn guest_without_host_and_duplicate_host_fail() {
        let addr = start();
        let code = RoomCode(*b"ABCD");
        let mut ghost = join(addr, Hello::Guest(code));
        let reply = read_frame(&mut ghost).unwrap();
        assert_eq!(reply, vec![ERROR, ERR_MISSING]);
        let mut host = join(addr, Hello::Host(code));
        assert_eq!(read_frame(&mut host).unwrap(), vec![JOINED, 0]);
        let mut dup = join(addr, Hello::Host(code));
        assert_eq!(read_frame(&mut dup).unwrap(), vec![ERROR, ERR_EXISTS]);
    }
}
