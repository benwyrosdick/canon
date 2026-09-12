use bevy::prelude::Vec3;

use crate::terrain::MapSize;

const WELCOME: u8 = 1;
const AIM: u8 = 2;
const READY: u8 = 3;
const FIRE: u8 = 4;
const SHOT: u8 = 5;
const BALL: u8 = 6;
const IMPACT: u8 = 7;
const MISS: u8 = 8;
const TURN: u8 = 9;
const NEW_MATCH: u8 = 10;

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    Welcome {
        seed: u64,
        size: MapSize,
    },
    Aim {
        yaw: f32,
        elevation: f32,
        power: f32,
    },
    Ready,
    Fire {
        yaw: f32,
        elevation: f32,
        power: f32,
    },
    ShotFired {
        yaw: f32,
        elevation: f32,
        power: f32,
    },
    Ball {
        position: Vec3,
        velocity: Vec3,
    },
    Impact {
        point: Vec3,
        damage: [f32; 2],
        health: [f32; 2],
        cannon_y: [f32; 2],
        direct: bool,
    },
    Miss {
        cannon_y: [f32; 2],
    },
    Turn {
        active: usize,
        round: u32,
        wind: Vec3,
        health: [f32; 2],
        cannon_y: [f32; 2],
        winner: Option<Option<usize>>,
    },
    NewMatch {
        seed: u64,
        size: MapSize,
    },
}

impl Msg {
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        match *self {
            Self::Welcome { seed, size } => {
                out.push(WELCOME);
                push_u64(&mut out, seed);
                out.push(size_byte(size));
            }
            Self::Aim {
                yaw,
                elevation,
                power,
            } => {
                out.push(AIM);
                push_f32(&mut out, yaw);
                push_f32(&mut out, elevation);
                push_f32(&mut out, power);
            }
            Self::Ready => out.push(READY),
            Self::Fire {
                yaw,
                elevation,
                power,
            } => {
                out.push(FIRE);
                push_f32(&mut out, yaw);
                push_f32(&mut out, elevation);
                push_f32(&mut out, power);
            }
            Self::ShotFired {
                yaw,
                elevation,
                power,
            } => {
                out.push(SHOT);
                push_f32(&mut out, yaw);
                push_f32(&mut out, elevation);
                push_f32(&mut out, power);
            }
            Self::Ball { position, velocity } => {
                out.push(BALL);
                push_vec3(&mut out, position);
                push_vec3(&mut out, velocity);
            }
            Self::Impact {
                point,
                damage,
                health,
                cannon_y,
                direct,
            } => {
                out.push(IMPACT);
                push_vec3(&mut out, point);
                push_f32(&mut out, damage[0]);
                push_f32(&mut out, damage[1]);
                push_f32(&mut out, health[0]);
                push_f32(&mut out, health[1]);
                push_f32(&mut out, cannon_y[0]);
                push_f32(&mut out, cannon_y[1]);
                out.push(u8::from(direct));
            }
            Self::Miss { cannon_y } => {
                out.push(MISS);
                push_f32(&mut out, cannon_y[0]);
                push_f32(&mut out, cannon_y[1]);
            }
            Self::Turn {
                active,
                round,
                wind,
                health,
                cannon_y,
                winner,
            } => {
                out.push(TURN);
                out.push(active as u8);
                push_u32(&mut out, round);
                push_vec3(&mut out, wind);
                push_f32(&mut out, health[0]);
                push_f32(&mut out, health[1]);
                push_f32(&mut out, cannon_y[0]);
                push_f32(&mut out, cannon_y[1]);
                out.push(match winner {
                    None => 0,
                    Some(None) => 1,
                    Some(Some(0)) => 2,
                    Some(Some(_)) => 3,
                });
            }
            Self::NewMatch { seed, size } => {
                out.push(NEW_MATCH);
                push_u64(&mut out, seed);
                out.push(size_byte(size));
            }
        }
        out
    }

    pub fn decode(bytes: &[u8]) -> Option<Self> {
        let mut i = 0;
        match take_u8(bytes, &mut i)? {
            WELCOME => Some(Self::Welcome {
                seed: take_u64(bytes, &mut i)?,
                size: map_size(take_u8(bytes, &mut i)?)?,
            }),
            AIM => Some(Self::Aim {
                yaw: take_f32(bytes, &mut i)?,
                elevation: take_f32(bytes, &mut i)?,
                power: take_f32(bytes, &mut i)?,
            }),
            READY => Some(Self::Ready),
            FIRE => Some(Self::Fire {
                yaw: take_f32(bytes, &mut i)?,
                elevation: take_f32(bytes, &mut i)?,
                power: take_f32(bytes, &mut i)?,
            }),
            SHOT => Some(Self::ShotFired {
                yaw: take_f32(bytes, &mut i)?,
                elevation: take_f32(bytes, &mut i)?,
                power: take_f32(bytes, &mut i)?,
            }),
            BALL => Some(Self::Ball {
                position: take_vec3(bytes, &mut i)?,
                velocity: take_vec3(bytes, &mut i)?,
            }),
            IMPACT => Some(Self::Impact {
                point: take_vec3(bytes, &mut i)?,
                damage: [take_f32(bytes, &mut i)?, take_f32(bytes, &mut i)?],
                health: [take_f32(bytes, &mut i)?, take_f32(bytes, &mut i)?],
                cannon_y: [take_f32(bytes, &mut i)?, take_f32(bytes, &mut i)?],
                direct: take_u8(bytes, &mut i)? != 0,
            }),
            MISS => Some(Self::Miss {
                cannon_y: [take_f32(bytes, &mut i)?, take_f32(bytes, &mut i)?],
            }),
            TURN => Some(Self::Turn {
                active: take_u8(bytes, &mut i)? as usize,
                round: take_u32(bytes, &mut i)?,
                wind: take_vec3(bytes, &mut i)?,
                health: [take_f32(bytes, &mut i)?, take_f32(bytes, &mut i)?],
                cannon_y: [take_f32(bytes, &mut i)?, take_f32(bytes, &mut i)?],
                winner: match take_u8(bytes, &mut i)? {
                    0 => None,
                    1 => Some(None),
                    2 => Some(Some(0)),
                    3 => Some(Some(1)),
                    _ => return None,
                },
            }),
            NEW_MATCH => Some(Self::NewMatch {
                seed: take_u64(bytes, &mut i)?,
                size: map_size(take_u8(bytes, &mut i)?)?,
            }),
            _ => None,
        }
        .filter(|_| i == bytes.len())
    }
}

fn size_byte(size: MapSize) -> u8 {
    match size {
        MapSize::Small => 0,
        MapSize::Medium => 1,
        MapSize::Large => 2,
    }
}

fn map_size(byte: u8) -> Option<MapSize> {
    Some(match byte {
        0 => MapSize::Small,
        1 => MapSize::Medium,
        2 => MapSize::Large,
        _ => return None,
    })
}

fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_f32(out: &mut Vec<u8>, value: f32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_vec3(out: &mut Vec<u8>, value: Vec3) {
    push_f32(out, value.x);
    push_f32(out, value.y);
    push_f32(out, value.z);
}

fn take_u8(data: &[u8], i: &mut usize) -> Option<u8> {
    let value = *data.get(*i)?;
    *i += 1;
    Some(value)
}

fn take_u32(data: &[u8], i: &mut usize) -> Option<u32> {
    let slice = data.get(*i..*i + 4)?;
    *i += 4;
    Some(u32::from_le_bytes(slice.try_into().ok()?))
}

fn take_u64(data: &[u8], i: &mut usize) -> Option<u64> {
    let slice = data.get(*i..*i + 8)?;
    *i += 8;
    Some(u64::from_le_bytes(slice.try_into().ok()?))
}

fn take_f32(data: &[u8], i: &mut usize) -> Option<f32> {
    let slice = data.get(*i..*i + 4)?;
    *i += 4;
    Some(f32::from_le_bytes(slice.try_into().ok()?))
}

fn take_vec3(data: &[u8], i: &mut usize) -> Option<Vec3> {
    Some(Vec3::new(
        take_f32(data, i)?,
        take_f32(data, i)?,
        take_f32(data, i)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_message_survives_a_round_trip() {
        let samples = [
            Msg::Welcome {
                seed: 42,
                size: MapSize::Large,
            },
            Msg::Aim {
                yaw: 1.25,
                elevation: 0.7,
                power: 33.5,
            },
            Msg::Ready,
            Msg::Fire {
                yaw: 0.1,
                elevation: 0.2,
                power: 40.0,
            },
            Msg::ShotFired {
                yaw: 0.1,
                elevation: 0.2,
                power: 40.0,
            },
            Msg::Ball {
                position: Vec3::new(1.0, 2.0, 3.0),
                velocity: Vec3::new(4.0, 5.0, 6.0),
            },
            Msg::Impact {
                point: Vec3::ZERO,
                damage: [10.0, 100.0],
                health: [90.0, 0.0],
                cannon_y: [4.0, 3.0],
                direct: true,
            },
            Msg::Miss {
                cannon_y: [5.0, 6.0],
            },
            Msg::Turn {
                active: 1,
                round: 3,
                wind: Vec3::X * 2.0,
                health: [80.0, 70.0],
                cannon_y: [1.0, 2.0],
                winner: None,
            },
            Msg::Turn {
                active: 0,
                round: 4,
                wind: Vec3::ZERO,
                health: [0.0, 0.0],
                cannon_y: [1.0, 1.0],
                winner: Some(None),
            },
            Msg::NewMatch {
                seed: 9,
                size: MapSize::Medium,
            },
        ];
        for msg in samples {
            assert_eq!(Msg::decode(&msg.encode()).as_ref(), Some(&msg));
        }
        assert!(Msg::decode(&[255]).is_none());
        assert!(Msg::decode(&[READY, 0]).is_none());
    }
}
