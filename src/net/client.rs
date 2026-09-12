use std::{
    io,
    net::{TcpStream, ToSocketAddrs},
    sync::{Mutex, mpsc},
    thread,
    time::Duration,
};

use super::wire::{self, ERROR, Hello, JOINED, PEER_JOINED, PEER_LEFT, PING, PONG};

pub enum Incoming {
    Joined { seat: u8 },
    PeerJoined,
    PeerLeft,
    Error(u8),
    Game(Vec<u8>),
    Closed,
}

pub struct Link {
    pub tx: mpsc::Sender<Vec<u8>>,
    pub rx: Mutex<mpsc::Receiver<Incoming>>,
}

impl Link {
    pub fn send_bytes(&self, payload: &[u8]) {
        let _ = self.tx.send(payload.to_vec());
    }
}

pub fn spawn(relay: String, hello: Hello) -> Link {
    let (out_tx, out_rx) = mpsc::channel();
    let (in_tx, in_rx) = mpsc::channel();
    thread::spawn(move || {
        let tx = in_tx.clone();
        if let Err(error) = drive(&relay, hello, out_rx, in_tx)
            && error.kind() != io::ErrorKind::UnexpectedEof
        {
            eprintln!("net: {error}");
        }
        let _ = tx.send(Incoming::Closed);
    });
    Link {
        tx: out_tx,
        rx: Mutex::new(in_rx),
    }
}

fn drive(
    relay: &str,
    hello: Hello,
    outgoing: mpsc::Receiver<Vec<u8>>,
    incoming: mpsc::Sender<Incoming>,
) -> io::Result<()> {
    let addr = relay
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "relay address"))?;
    let stream = TcpStream::connect_timeout(&addr, Duration::from_secs(4))?;
    stream.set_nodelay(true)?;
    stream.set_read_timeout(Some(Duration::from_secs(60)))?;
    let mut reader = stream.try_clone()?;
    let mut writer = stream;
    wire::write_frame(&mut writer, &hello.encode())?;
    let first = wire::read_frame(&mut reader)?;
    match first.first().copied() {
        Some(JOINED) if first.len() == 2 => {
            let _ = incoming.send(Incoming::Joined { seat: first[1] });
        }
        Some(ERROR) if first.len() >= 2 => {
            let _ = incoming.send(Incoming::Error(first[1]));
            return Ok(());
        }
        _ => {
            let _ = incoming.send(Incoming::Error(wire::ERR_BAD));
            return Ok(());
        }
    }
    let mut writer_clone = writer.try_clone()?;
    thread::spawn(move || {
        while let Ok(frame) = outgoing.recv() {
            if wire::write_frame(&mut writer_clone, &frame).is_err() {
                break;
            }
        }
    });
    loop {
        let frame = wire::read_frame(&mut reader)?;
        let event = match frame.first().copied() {
            Some(PING | PONG) => continue,
            Some(PEER_JOINED) => Incoming::PeerJoined,
            Some(PEER_LEFT) => Incoming::PeerLeft,
            Some(ERROR) if frame.len() >= 2 => Incoming::Error(frame[1]),
            _ => Incoming::Game(frame),
        };
        if incoming.send(event).is_err() {
            break;
        }
    }
    Ok(())
}
