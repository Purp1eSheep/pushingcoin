use rodio::mixer::Mixer;
use rodio::source::{SineWave, Source};
use rodio::{DeviceSinkBuilder, MixerDeviceSink, Player};
use std::time::Duration;

pub struct SoundEngine {
    _sink: Option<MixerDeviceSink>,
    mixer: Option<Mixer>,
    pub muted: bool,
}

impl SoundEngine {
    pub fn new() -> Self {
        match DeviceSinkBuilder::open_default_sink() {
            Ok(sink) => {
                let mixer = sink.mixer().clone();
                Self {
                    _sink: Some(sink),
                    mixer: Some(mixer),
                    muted: false,
                }
            }
            Err(e) => {
                log::warn!("Audio output device could not be opened: {:?}", e);
                Self {
                    _sink: None,
                    mixer: None,
                    muted: true,
                }
            }
        }
    }

    fn play_source<S>(&self, source: S)
    where
        S: Source<Item = f32> + Send + 'static,
    {
        if self.muted {
            return;
        }
        if let Some(mixer) = &self.mixer {
            let player = Player::connect_new(mixer);
            player.append(source);
            player.detach();
        }
    }

    /// Heavy metallic clack & ring when a solid coin is spawned/dropped
    pub fn play_coin_drop(&self) {
        // Low-mid solid metal impact body (heavy clack)
        let s_body = SineWave::new(320.0)
            .take_duration(Duration::from_millis(65))
            .amplify(0.35);
        // Resonant metallic body ring
        let s_ring = SineWave::new(760.0)
            .take_duration(Duration::from_millis(85))
            .amplify(0.22);
        // Crisp high coin ping
        let s_ping = SineWave::new(1380.0)
            .take_duration(Duration::from_millis(35))
            .amplify(0.12);

        self.play_source(s_body.mix(s_ring).mix(s_ping));
    }

    /// Pentatonic ascending reward chimes on scoring
    pub fn play_coin_score(&self, combo: u32) {
        let notes = [523.25, 659.25, 783.99, 987.77, 1046.50, 1318.51, 1567.98, 2093.00];
        let idx = (combo as usize).min(notes.len() - 1);
        let freq = notes[idx];

        let s1 = SineWave::new(freq)
            .take_duration(Duration::from_millis(110))
            .amplify(0.28);
        let s2 = SineWave::new(freq * 2.0)
            .take_duration(Duration::from_millis(60))
            .amplify(0.15);

        self.play_source(s1.mix(s2));
    }

    /// Special shimmering double-chime for Gem coins
    pub fn play_gem_score(&self) {
        let s1 = SineWave::new(1760.0)
            .take_duration(Duration::from_millis(120))
            .amplify(0.3);
        let s2 = SineWave::new(2637.0)
            .take_duration(Duration::from_millis(160))
            .amplify(0.2);
        self.play_source(s1.mix(s2));
    }

    /// Fanfare burst when Fever Mode activates!
    pub fn play_fever(&self) {
        if self.muted {
            return;
        }
        if let Some(mixer) = &self.mixer {
            let player = Player::connect_new(mixer);
            let s1 = SineWave::new(880.0)
                .take_duration(Duration::from_millis(90))
                .amplify(0.35);
            let s2 = SineWave::new(1174.66)
                .take_duration(Duration::from_millis(110))
                .amplify(0.35);
            let s3 = SineWave::new(1760.0)
                .take_duration(Duration::from_millis(220))
                .amplify(0.4);
            player.append(s1);
            player.append(s2);
            player.append(s3);
            player.detach();
        }
    }

    /// Tilt mechanical thud
    pub fn play_tilt(&self) {
        let s = SineWave::new(75.0)
            .take_duration(Duration::from_millis(200))
            .amplify(0.45);
        self.play_source(s);
    }
}
