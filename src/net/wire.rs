use std::io::{self, Read, Write};

pub const MAX_FRAME: usize = 4096;
pub const ALPHABET: &[u8] = b"23456789ABCDEFGHJKLMNPQRSTUVWXYZ";

pub const HOST: u8 = b'H';
pub const GUEST: u8 = b'G';
pub const JOINED: u8 = b'Y';
pub const ERROR: u8 = b'E';
pub const PEER_JOINED: u8 = b'+';
pub const PEER_LEFT: u8 = b'-';

pub const ERR_EXISTS: u8 = 1;
pub const ERR_MISSING: u8 = 2;
pub const ERR_FULL: u8 = 3;
pub const ERR_BAD: u8 = 4;
pub const ERR_BUSY: u8 = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RoomCode(pub [u8; 4]);

impl RoomCode {
    pub fn parse(text: &str) -> Option<Self> {
        let bytes: Vec<u8> = text
            .bytes()
            .filter(|b| !b.is_ascii_whitespace())
            .map(|b| b.to_ascii_uppercase())
            .collect();
        if bytes.len() != 4 {
            return None;
        }
        let mut code = [0; 4];
        for (i, b) in bytes.into_iter().enumerate() {
            if !ALPHABET.contains(&b) {
                return None;
            }
            code[i] = b;
        }
        Some(Self(code))
    }

    pub fn random(seed: u64) -> Self {
        let mut n = seed ^ 0xC0DE_A11E;
        let mut code = [0; 4];
        for slot in &mut code {
            n = n.wrapping_mul(6364136223846793005).wrapping_add(1);
            *slot = ALPHABET[(n as usize) % ALPHABET.len()];
        }
        Self(code)
    }

    pub fn as_str(&self) -> String {
        String::from_utf8_lossy(&self.0).into_owned()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hello {
    Host(RoomCode),
    Guest(RoomCode),
}

impl Hello {
    pub fn encode(self) -> Vec<u8> {
        match self {
            Self::Host(code) => vec![HOST, code.0[0], code.0[1], code.0[2], code.0[3]],
            Self::Guest(code) => vec![GUEST, code.0[0], code.0[1], code.0[2], code.0[3]],
        }
    }

    pub fn decode(bytes: &[u8]) -> Option<Self> {
        if bytes.len() != 5 {
            return None;
        }
        let code = RoomCode([bytes[1], bytes[2], bytes[3], bytes[4]]);
        if !code.0.iter().all(|b| ALPHABET.contains(b)) {
            return None;
        }
        match bytes[0] {
            HOST => Some(Self::Host(code)),
            GUEST => Some(Self::Guest(code)),
            _ => None,
        }
    }
}

pub fn error_text(code: u8) -> &'static str {
    match code {
        ERR_EXISTS => "That room already exists. Host a new code.",
        ERR_MISSING => "No host is waiting with that code.",
        ERR_FULL => "That room is already full.",
        ERR_BUSY => "Relay is full. Try again shortly.",
        _ => "Could not join the relay.",
    }
}

pub fn write_frame(writer: &mut impl Write, payload: &[u8]) -> io::Result<()> {
    if payload.len() > MAX_FRAME {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "frame too large",
        ));
    }
    writer.write_all(&(payload.len() as u32).to_le_bytes())?;
    writer.write_all(payload)?;
    writer.flush()
}

pub fn read_frame(reader: &mut impl Read) -> io::Result<Vec<u8>> {
    let mut len = [0u8; 4];
    reader.read_exact(&mut len)?;
    let n = u32::from_le_bytes(len) as usize;
    if n == 0 || n > MAX_FRAME {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "bad frame length",
        ));
    }
    let mut payload = vec![0u8; n];
    reader.read_exact(&mut payload)?;
    Ok(payload)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn room_codes_reject_ambiguous_and_wrong_length() {
        assert!(RoomCode::parse("7K2Q").is_some());
        assert!(RoomCode::parse("7k2q").is_some());
        assert_eq!(RoomCode::parse("7K2Q"), RoomCode::parse(" 7k2q "));
        assert!(RoomCode::parse("O0I1").is_none());
        assert!(RoomCode::parse("ABC").is_none());
        assert_eq!(RoomCode::random(1), RoomCode::random(1));
        assert_ne!(RoomCode::random(1), RoomCode::random(2));
    }

    #[test]
    fn frames_round_trip_and_reject_oversize() {
        let mut buf = Cursor::new(Vec::new());
        write_frame(&mut buf, b"hello").unwrap();
        buf.set_position(0);
        assert_eq!(read_frame(&mut buf).unwrap(), b"hello");
        assert!(write_frame(&mut Cursor::new(Vec::new()), &[0; MAX_FRAME + 1]).is_err());
        let mut bad = Cursor::new(5000u32.to_le_bytes().to_vec());
        assert!(read_frame(&mut bad).is_err());
        assert_eq!(
            Hello::decode(&Hello::Host(RoomCode(*b"7K2Q")).encode()),
            Some(Hello::Host(RoomCode(*b"7K2Q")))
        );
        assert_eq!(
            Hello::decode(&Hello::Guest(RoomCode(*b"ABCD")).encode()),
            Some(Hello::Guest(RoomCode(*b"ABCD")))
        );
        assert!(Hello::decode(&[HOST, b'O', b'0', b'I', b'1']).is_none());
    }
}
