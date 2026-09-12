use bevy::prelude::*;

#[derive(Resource)]
pub struct Sounds {
    pub fire: Handle<AudioSource>,
    pub impact: Handle<AudioSource>,
}

pub fn setup(mut commands: Commands, mut audio: ResMut<Assets<AudioSource>>) {
    commands.insert_resource(Sounds {
        fire: audio.add(synthesize(0.35, 95.0, 17)),
        impact: audio.add(synthesize(0.85, 48.0, 39)),
    });
}

/// Embedded, procedurally synthesized mono PCM: no external assets are needed.
fn synthesize(duration: f32, frequency: f32, seed: u64) -> AudioSource {
    let rate = 22050u32;
    let count = (duration * rate as f32) as u32;
    let data_len = count * 2;
    let mut bytes = Vec::with_capacity(44 + data_len as usize);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_len).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&rate.to_le_bytes());
    bytes.extend_from_slice(&(rate * 2).to_le_bytes());
    bytes.extend_from_slice(&2u16.to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_len.to_le_bytes());
    let mut rng = crate::terrain::Rng::new(seed);
    let mut noise = 0.0;
    for i in 0..count {
        let t = i as f32 / rate as f32;
        noise = noise * 0.65 + rng.range(-1.0, 1.0) * 0.35;
        let envelope = (1.0 - t / duration).powi(3) * (t * 500.0).min(1.0);
        let tone = (std::f32::consts::TAU * frequency * (t - 0.3 * t * t)).sin();
        let sample = ((tone * 0.35 + noise * 0.65) * envelope * 14000.0) as i16;
        bytes.extend_from_slice(&sample.to_le_bytes());
    }
    AudioSource {
        bytes: bytes.into(),
    }
}
