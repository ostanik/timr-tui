use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player, Source, source::Buffered};
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SoundError {
    #[error("Sound output stream error: {0}")]
    OutputStream(String),
    #[error("Sound file error: {0}")]
    File(String),
    #[error("Sound decoder error: {0}")]
    Decoder(String),
}

pub fn validate_sound_file(path: &PathBuf) -> Result<&PathBuf, SoundError> {
    // validate path
    if !path.exists() {
        let err = SoundError::File(format!("File not found: {:?}", path));
        return Err(err);
    };

    // Validate file extension
    path.extension()
        .and_then(|ext| ext.to_str())
        .filter(|ext| ["mp3", "wav"].contains(&ext.to_lowercase().as_str()))
        .ok_or_else(|| {
            SoundError::File(
                "Unsupported file extension. Only .mp3 and .wav are supported".to_owned(),
            )
        })?;

    Ok(path)
}

pub struct Sound {
    buffer: Arc<Buffered<Decoder<BufReader<File>>>>,
    // Kept alive so the audio device stays open for the player's lifetime.
    _stream: MixerDeviceSink,
    player: Player,
}

impl Sound {
    pub fn new(path: PathBuf) -> Result<Self, SoundError> {
        let stream = DeviceSinkBuilder::open_default_sink()
            .map_err(|e: rodio::DeviceSinkError| SoundError::OutputStream(e.to_string()))?;

        let file = File::open(&path).map_err(|e| SoundError::File(e.to_string()))?;
        let decoder = Decoder::try_from(file).map_err(|e| SoundError::Decoder(e.to_string()))?;
        let buffer = Arc::new(decoder.buffered());
        let player = Player::connect_new(stream.mixer());

        Ok(Self {
            buffer,
            _stream: stream,
            player,
        })
    }

    /// Plays the sound in a loop until `stop` is called.
    pub fn play(&self) -> Result<(), SoundError> {
        self.player.stop();
        self.player.append((*self.buffer).clone().repeat_infinite());
        self.player.play();
        Ok(())
    }

    pub fn stop(&self) {
        self.player.stop();
    }
}
