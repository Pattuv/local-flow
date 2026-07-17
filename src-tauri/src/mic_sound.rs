use std::io::Cursor;
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, OnceLock};

use rodio::{buffer::SamplesBuffer, Decoder, OutputStream, Sink, Source};

const MIC_VOLUME: f32 = 0.15;

static PLAY_TX: OnceLock<Sender<()>> = OnceLock::new();

pub fn init() {
    let (ready_tx, ready_rx) = mpsc::channel();
    let (play_tx, play_rx) = mpsc::channel();

    std::thread::spawn(move || {
        let Ok((stream, handle)) = OutputStream::try_default() else {
            eprintln!("LocalFlow: failed to open audio output for mic sound");
            return;
        };

        let cursor = Cursor::new(include_bytes!("../../src/assets/sounds/mic.wav"));
        let Ok(decoder) = Decoder::new(cursor) else {
            eprintln!("LocalFlow: failed to decode mic.wav");
            return;
        };

        let channels = decoder.channels();
        let sample_rate = decoder.sample_rate();
        let samples: Arc<Vec<f32>> = Arc::new(decoder.convert_samples().collect());

        let Ok(sink) = Sink::try_new(&handle) else {
            eprintln!("LocalFlow: failed to create audio sink for mic sound");
            return;
        };
        sink.set_volume(MIC_VOLUME);

        let _ = ready_tx.send(());

        for _ in play_rx {
            sink.stop();
            sink.append(SamplesBuffer::new(
                channels,
                sample_rate,
                samples.as_ref().clone(),
            ));
        }

        drop(stream);
    });

    if ready_rx.recv().is_ok() {
        let _ = PLAY_TX.set(play_tx);
    }
}

pub fn play() {
    if let Some(tx) = PLAY_TX.get() {
        let _ = tx.send(());
    }
}
