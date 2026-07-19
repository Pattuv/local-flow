use std::io::Cursor;
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, OnceLock};

use rodio::{buffer::SamplesBuffer, Decoder, OutputStream, Sink, Source};

const MIC_VOLUME: f32 = 0.15;
const ERROR_VOLUME: f32 = 0.28;

#[derive(Clone, Copy)]
enum SoundId {
    Mic,
    Error,
}

struct LoadedSound {
    channels: u16,
    sample_rate: u32,
    samples: Arc<Vec<f32>>,
    volume: f32,
}

static PLAY_TX: OnceLock<Sender<SoundId>> = OnceLock::new();

fn decode_sound(bytes: &'static [u8], volume: f32) -> Option<LoadedSound> {
    let cursor = Cursor::new(bytes);
    let decoder = Decoder::new(cursor).ok()?;

    Some(LoadedSound {
        channels: decoder.channels(),
        sample_rate: decoder.sample_rate(),
        samples: Arc::new(decoder.convert_samples().collect()),
        volume,
    })
}

pub fn init() {
    let (ready_tx, ready_rx) = mpsc::channel();
    let (play_tx, play_rx) = mpsc::channel();

    std::thread::spawn(move || {
        let Ok((stream, handle)) = OutputStream::try_default() else {
            eprintln!("LocalFlow: failed to open audio output");
            return;
        };

        let Some(mic) = decode_sound(
            include_bytes!("../../src/assets/sounds/mic.wav"),
            MIC_VOLUME,
        ) else {
            eprintln!("LocalFlow: failed to decode mic.wav");
            return;
        };

        let Some(error) = decode_sound(
            include_bytes!("../../src/assets/sounds/error.wav"),
            ERROR_VOLUME,
        ) else {
            eprintln!("LocalFlow: failed to decode error.wav");
            return;
        };

        let Ok(sink) = Sink::try_new(&handle) else {
            eprintln!("LocalFlow: failed to create audio sink");
            return;
        };

        let _ = ready_tx.send(());

        for sound_id in play_rx {
            let sound = match sound_id {
                SoundId::Mic => &mic,
                SoundId::Error => &error,
            };

            sink.set_volume(sound.volume);
            sink.stop();
            sink.append(SamplesBuffer::new(
                sound.channels,
                sound.sample_rate,
                sound.samples.as_ref().clone(),
            ));
        }

        drop(stream);
    });

    if ready_rx.recv().is_ok() {
        let _ = PLAY_TX.set(play_tx);
    }
}

fn play(sound_id: SoundId) {
    if let Some(tx) = PLAY_TX.get() {
        let _ = tx.send(sound_id);
    }
}

pub fn play_mic() {
    play(SoundId::Mic);
}

pub fn play_error() {
    play(SoundId::Error);
}
