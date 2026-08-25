use std::f32::consts::TAU;
use std::num::NonZero;

use rodio::{buffer::SamplesBuffer, DeviceSinkBuilder, MixerDeviceSink, Player, Source};

const SAMPLE_RATE: u32 = 44_100;
const CHANNELS: u16 = 2;
const CITY_AMBIENCE_SECONDS: f32 = 40.0;

pub const BACKROOMS_ABC: &str = r#"
X:1
T:Fluorescent Rooms
L:1/8
Q:54
K:C
z8 C,6 z2 _E,4 z4 G,,8 z8 ^F,2 z2 C,4 z8
z4 _B,,6 z2 F,4 z6 C,,8 z8
"#;
pub const GLASS_STAIRCASE_ABC: &str = include_str!("../assets/glass_staircase.abc");
pub const ROWS_THAT_MOVE_ABC: &str = include_str!("../assets/rows_that_move.abc");

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NoteEvent {
    pub frequency_hz: Option<f32>,
    pub duration_seconds: f32,
    pub velocity: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AbcTune {
    pub title: Option<String>,
    pub tempo: f32,
    pub unit_note_seconds: f32,
    pub events: Vec<NoteEvent>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SoundEffect {
    Gunshot,
    EnemyHit,
    EnemyDeath,
    PlayerHurt,
}

pub struct GameAudio {
    backend: Option<RodioBackend>,
    ambience: Option<AmbienceKind>,
}

impl GameAudio {
    pub fn open() -> Self {
        Self {
            backend: RodioBackend::open().ok(),
            ambience: None,
        }
    }

    pub fn silent() -> Self {
        Self {
            backend: None,
            ambience: None,
        }
    }

    pub fn enter_city_mode(&mut self) {
        self.set_ambience(AmbienceKind::City);
    }

    pub fn enter_corn_maze_mode(&mut self) {
        self.set_ambience(AmbienceKind::CornMaze);
    }

    pub fn enter_doom_mode(&mut self) {
        self.set_ambience(AmbienceKind::Doom);
    }

    pub fn leave_ambience(&mut self) {
        self.ambience = None;
        if let Some(backend) = &mut self.backend {
            backend.stop_ambience();
        }
    }

    pub fn leave_doom_mode(&mut self) {
        self.leave_ambience();
    }

    fn set_ambience(&mut self, ambience: AmbienceKind) {
        if self.ambience == Some(ambience) {
            return;
        }

        self.ambience = Some(ambience);
        if let Some(backend) = &mut self.backend {
            match ambience {
                AmbienceKind::City => backend.start_city_ambience(),
                AmbienceKind::CornMaze => backend.start_corn_maze_ambience(),
                AmbienceKind::Doom => backend.start_backrooms_ambience(),
            }
        }
    }

    pub fn play_effect(&self, effect: SoundEffect) {
        if let Some(backend) = &self.backend {
            backend.play_effect(effect);
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.backend.is_some()
    }

    pub fn in_doom_mode(&self) -> bool {
        self.ambience == Some(AmbienceKind::Doom)
    }

    pub fn in_city_mode(&self) -> bool {
        self.ambience == Some(AmbienceKind::City)
    }

    pub fn in_corn_maze_mode(&self) -> bool {
        self.ambience == Some(AmbienceKind::CornMaze)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AmbienceKind {
    City,
    CornMaze,
    Doom,
}

struct RodioBackend {
    sink: MixerDeviceSink,
    ambience: Option<Player>,
}

impl RodioBackend {
    fn open() -> Result<Self, Box<dyn std::error::Error>> {
        let mut sink = DeviceSinkBuilder::open_default_sink()?;
        sink.log_on_drop(false);
        Ok(Self {
            sink,
            ambience: None,
        })
    }

    fn start_city_ambience(&mut self) {
        self.stop_ambience();
        let Ok(tune) = parse_abc(GLASS_STAIRCASE_ABC) else {
            return;
        };

        let samples = synthesize_abc_limited(&tune, 0.18, CITY_AMBIENCE_SECONDS);
        let player = Player::connect_new(self.sink.mixer());
        player.set_volume(0.48);
        player.append(samples_buffer(samples).repeat_infinite());
        self.ambience = Some(player);
    }

    fn start_corn_maze_ambience(&mut self) {
        self.stop_ambience();
        let Ok(tune) = parse_abc(ROWS_THAT_MOVE_ABC) else {
            return;
        };

        let samples = synthesize_abc_limited(&tune, 0.16, CITY_AMBIENCE_SECONDS);
        let player = Player::connect_new(self.sink.mixer());
        player.set_volume(0.5);
        player.append(samples_buffer(samples).repeat_infinite());
        self.ambience = Some(player);
    }

    fn start_backrooms_ambience(&mut self) {
        self.stop_ambience();
        let Ok(tune) = parse_abc(BACKROOMS_ABC) else {
            return;
        };

        let mut samples = synthesize_backrooms_bed(24.0);
        mix_in(&mut samples, &synthesize_abc(&tune, 0.22), 0);

        let player = Player::connect_new(self.sink.mixer());
        player.set_volume(0.55);
        player.append(samples_buffer(samples).repeat_infinite());
        self.ambience = Some(player);
    }

    fn stop_ambience(&mut self) {
        if let Some(player) = self.ambience.take() {
            player.stop();
        }
    }

    fn play_effect(&self, effect: SoundEffect) {
        let samples = match effect {
            SoundEffect::Gunshot => synthesize_gunshot(),
            SoundEffect::EnemyHit => synthesize_enemy_hit(),
            SoundEffect::EnemyDeath => synthesize_enemy_death(),
            SoundEffect::PlayerHurt => synthesize_player_hurt(),
        };
        self.sink.mixer().add(samples_buffer(samples));
    }
}

pub fn parse_abc(input: &str) -> Result<AbcTune, String> {
    let mut title = None;
    let mut tempo = 120.0;
    let mut unit_note_value = None;
    let mut body = String::new();

    for raw_line in input.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('%') {
            continue;
        }

        if let Some(value) = line.strip_prefix("T:") {
            title = Some(value.trim().to_string());
        } else if let Some(value) = line.strip_prefix("Q:") {
            tempo = parse_tempo(value.trim()).unwrap_or(tempo);
        } else if let Some(value) = line.strip_prefix("L:") {
            let value = value.trim();
            parse_fraction(value).ok_or_else(|| format!("invalid L: field: {value}"))?;
            unit_note_value = Some(value.to_string());
        } else if line.len() >= 2 && line.as_bytes()[1] == b':' {
            continue;
        } else {
            body.push_str(line);
            body.push(' ');
        }
    }

    if body.trim().is_empty() {
        return Err("ABC tune has no note body".to_string());
    }

    let unit_note_seconds = unit_note_value
        .as_deref()
        .and_then(|value| parse_unit_note_seconds(value, tempo))
        .unwrap_or(60.0 / tempo);

    Ok(AbcTune {
        title,
        tempo,
        unit_note_seconds,
        events: parse_events(&body, unit_note_seconds)?,
    })
}

fn parse_tempo(value: &str) -> Option<f32> {
    value
        .split(|ch: char| !(ch.is_ascii_digit() || ch == '.'))
        .filter(|part| !part.is_empty())
        .last()
        .and_then(|part| part.parse::<f32>().ok())
        .filter(|tempo| *tempo > 0.0)
}

fn parse_unit_note_seconds(value: &str, tempo: f32) -> Option<f32> {
    let (_numerator, _denominator) = parse_fraction(value)?;
    Some(60.0 / tempo)
}

fn parse_events(body: &str, unit_note_seconds: f32) -> Result<Vec<NoteEvent>, String> {
    let mut chars = body.chars().peekable();
    let mut events = Vec::new();

    while let Some(ch) = chars.peek().copied() {
        match ch {
            ' ' | '\t' | '\r' | '\n' | '|' | ':' => {
                chars.next();
            }
            '"' => skip_until(&mut chars, '"'),
            '[' => skip_until(&mut chars, ']'),
            '^' | '_' | '=' | 'A'..='G' | 'a'..='g' | 'z' => {
                let semitone_offset = parse_accidental(&mut chars);
                let Some(symbol) = chars.next() else {
                    break;
                };
                if symbol == 'z' {
                    let duration = parse_length_multiplier(&mut chars) * unit_note_seconds;
                    events.push(NoteEvent {
                        frequency_hz: None,
                        duration_seconds: duration,
                        velocity: 0.0,
                    });
                    continue;
                }

                if !symbol.is_ascii_alphabetic() {
                    return Err(format!("expected note, got {symbol}"));
                }

                let mut octave_shift = 0;
                while let Some(mark) = chars.peek().copied() {
                    match mark {
                        '\'' => {
                            octave_shift += 1;
                            chars.next();
                        }
                        ',' => {
                            octave_shift -= 1;
                            chars.next();
                        }
                        _ => break,
                    }
                }

                let duration = parse_length_multiplier(&mut chars) * unit_note_seconds;
                events.push(NoteEvent {
                    frequency_hz: Some(note_frequency(symbol, semitone_offset, octave_shift)),
                    duration_seconds: duration,
                    velocity: 0.72,
                });
            }
            _ => return Err(format!("unsupported ABC token: {ch}")),
        }
    }

    Ok(events)
}

fn skip_until<I>(chars: &mut std::iter::Peekable<I>, terminator: char)
where
    I: Iterator<Item = char>,
{
    chars.next();
    for ch in chars.by_ref() {
        if ch == terminator {
            break;
        }
    }
}

fn parse_accidental<I>(chars: &mut std::iter::Peekable<I>) -> i32
where
    I: Iterator<Item = char>,
{
    let mut offset = 0;
    while let Some(ch) = chars.peek().copied() {
        match ch {
            '^' => {
                offset += 1;
                chars.next();
            }
            '_' => {
                offset -= 1;
                chars.next();
            }
            '=' => {
                offset = 0;
                chars.next();
            }
            _ => break,
        }
    }
    offset
}

fn parse_length_multiplier<I>(chars: &mut std::iter::Peekable<I>) -> f32
where
    I: Iterator<Item = char>,
{
    let numerator = parse_digits(chars).unwrap_or(1.0);
    if chars.peek() != Some(&'/') {
        return numerator;
    }

    chars.next();
    let denominator = parse_digits(chars).unwrap_or(2.0);
    numerator / denominator.max(1.0)
}

fn parse_digits<I>(chars: &mut std::iter::Peekable<I>) -> Option<f32>
where
    I: Iterator<Item = char>,
{
    let mut digits = String::new();
    while let Some(ch) = chars.peek().copied() {
        if ch.is_ascii_digit() {
            digits.push(ch);
            chars.next();
        } else {
            break;
        }
    }
    if digits.is_empty() {
        None
    } else {
        digits.parse().ok()
    }
}

fn parse_fraction(value: &str) -> Option<(f32, f32)> {
    if let Some((num, den)) = value.split_once('/') {
        Some((num.trim().parse().ok()?, den.trim().parse().ok()?))
    } else {
        Some((value.trim().parse().ok()?, 1.0))
    }
}

fn note_frequency(note: char, accidental: i32, octave_shift: i32) -> f32 {
    let base = match note.to_ascii_uppercase() {
        'C' => 60,
        'D' => 62,
        'E' => 64,
        'F' => 65,
        'G' => 67,
        'A' => 69,
        'B' => 71,
        _ => 60,
    };
    let lowercase_octave = if note.is_ascii_lowercase() { 12 } else { 0 };
    let midi = base + accidental + octave_shift * 12 + lowercase_octave;
    440.0 * 2.0_f32.powf((midi as f32 - 69.0) / 12.0)
}

pub fn synthesize_abc(tune: &AbcTune, gain: f32) -> Vec<f32> {
    synthesize_abc_limited(tune, gain, f32::INFINITY)
}

fn synthesize_abc_limited(tune: &AbcTune, gain: f32, max_seconds: f32) -> Vec<f32> {
    let mut samples = Vec::new();
    let mut elapsed = 0.0;
    for event in &tune.events {
        if elapsed >= max_seconds {
            break;
        }

        let duration_seconds = event.duration_seconds.min(max_seconds - elapsed);
        let frames = (duration_seconds * SAMPLE_RATE as f32).max(1.0) as usize;
        for frame in 0..frames {
            let t = frame as f32 / SAMPLE_RATE as f32;
            let envelope = envelope(frame, frames, 0.08, 0.22);
            let sample = if let Some(freq) = event.frequency_hz {
                let detuned = (TAU * (freq * 0.997) * t).sin() * 0.45;
                ((TAU * freq * t).sin() * 0.55 + detuned) * event.velocity * gain * envelope
            } else {
                0.0
            };
            push_stereo(&mut samples, sample, sample);
        }
        elapsed += duration_seconds;
    }
    samples
}

pub fn synthesize_backrooms_bed(duration_seconds: f32) -> Vec<f32> {
    let frames = (duration_seconds * SAMPLE_RATE as f32).max(1.0) as usize;
    let mut samples = Vec::with_capacity(frames * CHANNELS as usize);
    let mut noise = 0xBADC_0FFEu32;

    for frame in 0..frames {
        let t = frame as f32 / SAMPLE_RATE as f32;
        noise = xorshift(noise);
        let white = ((noise >> 9) as f32 / (1_u32 << 23) as f32) * 2.0 - 1.0;
        let drone = (TAU * 57.0 * t).sin() * 0.10 + (TAU * 61.4 * t).sin() * 0.07;
        let fluorescent = (TAU * 119.7 * t).sin() * 0.035 + (TAU * 239.5 * t).sin() * 0.018;
        let flicker = if (t * 8.0).sin() > 0.92 { 0.055 } else { 0.0 };
        let distant = (TAU * 0.071 * t).sin() * (TAU * 184.0 * t).sin() * 0.025;
        let sample = soft_clip(drone + fluorescent + white * 0.018 + flicker + distant);
        push_stereo(&mut samples, sample * 0.92, sample * 0.82);
    }

    samples
}

pub fn synthesize_effect(effect: SoundEffect) -> Vec<f32> {
    match effect {
        SoundEffect::Gunshot => synthesize_gunshot(),
        SoundEffect::EnemyHit => synthesize_enemy_hit(),
        SoundEffect::EnemyDeath => synthesize_enemy_death(),
        SoundEffect::PlayerHurt => synthesize_player_hurt(),
    }
}

fn synthesize_gunshot() -> Vec<f32> {
    synthesize_burst(0.23, 96.0, 1_900.0, 0.95)
}

fn synthesize_enemy_hit() -> Vec<f32> {
    synthesize_burst(0.16, 180.0, 720.0, 0.55)
}

fn synthesize_enemy_death() -> Vec<f32> {
    synthesize_burst(0.42, 88.0, 260.0, 0.62)
}

fn synthesize_player_hurt() -> Vec<f32> {
    synthesize_burst(0.28, 52.0, 140.0, 0.70)
}

fn synthesize_burst(duration_seconds: f32, low_hz: f32, high_hz: f32, gain: f32) -> Vec<f32> {
    let frames = (duration_seconds * SAMPLE_RATE as f32).max(1.0) as usize;
    let mut samples = Vec::with_capacity(frames * CHANNELS as usize);
    let mut noise = 0x51F7_A110u32;

    for frame in 0..frames {
        let t = frame as f32 / SAMPLE_RATE as f32;
        let progress = frame as f32 / frames as f32;
        let sweep = high_hz + (low_hz - high_hz) * progress;
        noise = xorshift(noise);
        let white = ((noise >> 9) as f32 / (1_u32 << 23) as f32) * 2.0 - 1.0;
        let tone = (TAU * sweep * t).sin();
        let decay = (1.0 - progress).powf(3.2);
        let thump = (TAU * low_hz * t).sin() * (1.0 - progress).powf(5.0);
        let sample = soft_clip((white * 0.7 + tone * 0.25 + thump * 1.2) * gain * decay);
        push_stereo(&mut samples, sample, sample * 0.93);
    }

    samples
}

fn samples_buffer(samples: Vec<f32>) -> SamplesBuffer {
    SamplesBuffer::new(
        NonZero::new(CHANNELS).unwrap(),
        NonZero::new(SAMPLE_RATE).unwrap(),
        samples,
    )
}

fn mix_in(base: &mut [f32], overlay: &[f32], offset: usize) {
    if offset >= base.len() {
        return;
    }
    for (dst, src) in base[offset..].iter_mut().zip(overlay.iter()) {
        *dst = soft_clip(*dst + *src);
    }
}

fn push_stereo(samples: &mut Vec<f32>, left: f32, right: f32) {
    samples.push(soft_clip(left));
    samples.push(soft_clip(right));
}

fn envelope(frame: usize, frames: usize, attack: f32, release: f32) -> f32 {
    let progress = frame as f32 / frames.max(1) as f32;
    let attack_gain = (progress / attack.max(0.001)).clamp(0.0, 1.0);
    let release_gain = ((1.0 - progress) / release.max(0.001)).clamp(0.0, 1.0);
    attack_gain.min(release_gain)
}

fn soft_clip(sample: f32) -> f32 {
    sample.clamp(-1.0, 1.0).tanh()
}

fn xorshift(mut value: u32) -> u32 {
    value ^= value << 13;
    value ^= value >> 17;
    value ^= value << 5;
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_abc_headers_and_notes() {
        let tune = parse_abc("X:1\nT:Test\nL:1/8\nQ:120\nK:C\nC D E z2\n").unwrap();

        assert_eq!(tune.title.as_deref(), Some("Test"));
        assert_eq!(tune.events.len(), 4);
        assert!(tune.events[0].frequency_hz.unwrap() > 260.0);
        assert!(tune.events[0].duration_seconds > 0.24);
        assert_eq!(tune.events[3].frequency_hz, None);
        assert!(tune.events[3].duration_seconds > tune.events[0].duration_seconds);
    }

    #[test]
    fn parses_tempo_ratio_chord_annotations_and_inline_headers() {
        let tune = parse_abc("L:1/8\nQ:1/4=112\nK:Em\n\"Em9\"E2 G | [M:5/8] \"B7\"^D F\n").unwrap();

        assert_eq!(tune.tempo, 112.0);
        assert_eq!(tune.events.len(), 4);
        assert!(tune.events.iter().all(|event| event.frequency_hz.is_some()));
    }

    #[test]
    fn parses_glass_staircase_asset() {
        let tune = parse_abc(GLASS_STAIRCASE_ABC).unwrap();

        assert_eq!(tune.title.as_deref(), Some("Glass Staircase"));
        assert!(tune.events.len() > 250);
        assert!(tune
            .events
            .iter()
            .filter_map(|event| event.frequency_hz)
            .any(|freq| freq > 1_000.0));
    }

    #[test]
    fn parses_rows_that_move_asset() {
        let tune = parse_abc(ROWS_THAT_MOVE_ABC).unwrap();

        assert_eq!(tune.title.as_deref(), Some("Rows That Move"));
        assert_eq!(tune.tempo, 76.0);
        assert!(tune.events.len() > 150);
        assert!(
            tune.events
                .iter()
                .filter(|event| event.frequency_hz.is_none())
                .count()
                > 10
        );
    }

    #[test]
    fn parses_accidentals_octaves_and_fractional_lengths() {
        let tune = parse_abc("L:1/4\nQ:60\nK:C\n^C,3/2 _d/2 =F\n").unwrap();

        assert_eq!(tune.events.len(), 3);
        assert!(tune.events[0].frequency_hz.unwrap() < tune.events[2].frequency_hz.unwrap());
        assert!(tune.events[1].frequency_hz.unwrap() > 500.0);
        assert!((tune.events[0].duration_seconds - 1.5).abs() < 0.001);
        assert!((tune.events[1].duration_seconds - 0.5).abs() < 0.001);
    }

    #[test]
    fn backrooms_abc_produces_sparse_low_tune() {
        let tune = parse_abc(BACKROOMS_ABC).unwrap();
        let note_count = tune
            .events
            .iter()
            .filter(|event| event.frequency_hz.is_some())
            .count();
        let rest_count = tune
            .events
            .iter()
            .filter(|event| event.frequency_hz.is_none())
            .count();

        assert!(note_count > 5);
        assert!(rest_count >= note_count);
        assert!(tune
            .events
            .iter()
            .filter_map(|event| event.frequency_hz)
            .any(|freq| freq < 140.0));
    }

    #[test]
    fn synthesizes_stereo_effect_buffers() {
        let gunshot = synthesize_effect(SoundEffect::Gunshot);
        let hurt = synthesize_effect(SoundEffect::PlayerHurt);

        assert!(gunshot.len() > hurt.len() / 2);
        assert_eq!(gunshot.len() % CHANNELS as usize, 0);
        assert!(gunshot.iter().any(|sample| sample.abs() > 0.01));
    }

    #[test]
    fn silent_audio_tracks_mode_without_device() {
        let mut audio = GameAudio::silent();

        audio.enter_doom_mode();
        audio.play_effect(SoundEffect::Gunshot);
        assert!(audio.in_doom_mode());

        audio.leave_doom_mode();
        assert!(!audio.in_doom_mode());

        audio.enter_city_mode();
        assert!(audio.in_city_mode());

        audio.enter_corn_maze_mode();
        assert!(audio.in_corn_maze_mode());
        assert!(!audio.in_city_mode());

        audio.leave_ambience();
        assert!(!audio.in_city_mode());
        assert!(!audio.in_corn_maze_mode());
    }
}
