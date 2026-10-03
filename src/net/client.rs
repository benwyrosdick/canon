use std::{
    io,
    net::{IpAddr, SocketAddr, TcpStream, ToSocketAddrs},
    sync::{Mutex, mpsc},
    thread,
    time::{Duration, Instant},
};

/// How often the writer pings the relay; the relay answers each with a pong.
/// Sent from the network thread so it keeps going if the game loop stalls.
const PING_EVERY: Duration = Duration::from_secs(5);
/// Silence long enough to miss a few pongs means the relay is gone, even if
/// the socket never closed (e.g. the host machine was suspended).
const SILENCE_LIMIT: Duration = Duration::from_secs(15);
const CONNECT_ATTEMPT: Duration = Duration::from_secs(3);
const CONNECT_BUDGET: Duration = Duration::from_secs(15);
const WAKE_TIMEOUT: Duration = Duration::from_secs(5);

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
        match drive(&relay, hello, out_rx, in_tx) {
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                ) =>
            {
                eprintln!("net: relay stopped responding");
            }
            Err(error) if error.kind() != io::ErrorKind::UnexpectedEof => {
                eprintln!("net: {error}");
            }
            _ => {}
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
    wake(relay);
    let stream = connect(addr)?;
    stream.set_nodelay(true)?;
    stream.set_read_timeout(Some(SILENCE_LIMIT))?;
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
        let mut next_ping = Instant::now() + PING_EVERY;
        loop {
            let wait = next_ping.saturating_duration_since(Instant::now());
            let frame = match outgoing.recv_timeout(wait) {
                Ok(frame) => frame,
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    next_ping = Instant::now() + PING_EVERY;
                    vec![PING]
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            };
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

/// Sleeping hosts (e.g. a suspended boxd machine) only wake on HTTPS, not on
/// the relay's raw TCP port, so poke the relay's hostname first. Best effort:
/// any response, even an error status, means the host is awake.
fn wake(relay: &str) {
    let Some(host) = wake_host(relay) else {
        return;
    };
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(WAKE_TIMEOUT))
        .http_status_as_error(false)
        .build()
        .into();
    if let Err(error) = agent.get(format!("https://{host}/")).call() {
        eprintln!("net: wake {host}: {error}");
    }
}

fn wake_host(relay: &str) -> Option<&str> {
    let (host, _port) = relay.rsplit_once(':')?;
    let host = host.trim_start_matches('[').trim_end_matches(']');
    if host.is_empty() || host.eq_ignore_ascii_case("localhost") || host.parse::<IpAddr>().is_ok() {
        return None;
    }
    Some(host)
}

/// A waking host can take a moment before its forwarded port accepts, so keep
/// retrying until the budget runs out.
fn connect(addr: SocketAddr) -> io::Result<TcpStream> {
    let deadline = Instant::now() + CONNECT_BUDGET;
    loop {
        match TcpStream::connect_timeout(&addr, CONNECT_ATTEMPT) {
            Ok(stream) => return Ok(stream),
            Err(error) if Instant::now() + CONNECT_ATTEMPT > deadline => return Err(error),
            Err(_) => thread::sleep(Duration::from_millis(500)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::wake_host;

    #[test]
    fn wakes_named_hosts_only() {
        assert_eq!(wake_host("canon.boxd.benwyrosdick.com:59222"), Some("canon.boxd.benwyrosdick.com"));
        assert_eq!(wake_host("127.0.0.1:3478"), None);
        assert_eq!(wake_host("[::1]:3478"), None);
        assert_eq!(wake_host("localhost:3478"), None);
        assert_eq!(wake_host("no-port"), None);
    }
}
