use crate::common::Toggle;
use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player, Source, source::Buffered};
use std::borrow::Cow;
use std::io::Cursor;
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;

const DEFAULT_CHIME: &[u8] = include_bytes!("../assets/chime.mp3");

type SoundData = Cursor<Cow<'static, [u8]>>;

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

/// Value of the `--sound` argument: `on`, `off` or a path to a custom sound file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SoundArg {
    Toggle(Toggle),
    Path(PathBuf),
}

pub fn parse_sound_arg(s: &str) -> Result<SoundArg, SoundError> {
    match s.to_lowercase().as_str() {
        "on" => Ok(SoundArg::Toggle(Toggle::On)),
        "off" => Ok(SoundArg::Toggle(Toggle::Off)),
        _ => {
            let path = PathBuf::from(s);
            validate_sound_file(&path)?;
            Ok(SoundArg::Path(path))
        }
    }
}

fn decode(bytes: Cow<'static, [u8]>) -> Result<Decoder<SoundData>, SoundError> {
    Decoder::new(Cursor::new(bytes)).map_err(|e| SoundError::Decoder(e.to_string()))
}

pub struct Sound {
    buffer: Arc<Buffered<Decoder<SoundData>>>,
    // Kept alive so the audio device stays open for the player's lifetime.
    _stream: MixerDeviceSink,
    player: Player,
}

impl Sound {
    /// Creates a sound from a custom file, or from the built-in chime if no path is given.
    pub fn new(path: Option<PathBuf>) -> Result<Self, SoundError> {
        let stream = DeviceSinkBuilder::open_default_sink()
            .map_err(|e: rodio::DeviceSinkError| SoundError::OutputStream(e.to_string()))?;

        let bytes = match path {
            Some(path) => {
                Cow::Owned(std::fs::read(path).map_err(|e| SoundError::File(e.to_string()))?)
            }
            None => Cow::Borrowed(DEFAULT_CHIME),
        };
        let buffer = Arc::new(decode(bytes)?.buffered());
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_chime_decodes() {
        assert!(decode(Cow::Borrowed(DEFAULT_CHIME)).is_ok());
    }

    #[test]
    fn parse_on_off() {
        assert_eq!(parse_sound_arg("on").unwrap(), SoundArg::Toggle(Toggle::On));
        assert_eq!(
            parse_sound_arg("OFF").unwrap(),
            SoundArg::Toggle(Toggle::Off)
        );
    }

    #[test]
    fn parse_path() {
        assert_eq!(
            parse_sound_arg("assets/chime.mp3").unwrap(),
            SoundArg::Path(PathBuf::from("assets/chime.mp3"))
        );
    }

    #[test]
    fn parse_missing_file_fails() {
        assert!(parse_sound_arg("assets/does-not-exist.mp3").is_err());
    }
}
