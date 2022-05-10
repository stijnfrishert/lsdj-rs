//! The `render` subcommand

use crate::utils::check_for_overwrite;
use anyhow::{Context, Result};
use clap::Args;
use humantime::parse_duration;
use mizu_core::{AudioBuffers, GameBoy, GameboyConfig, JoypadButton};
use std::{
    fs::File,
    path::{Path, PathBuf},
};
use wav::{header::WAV_FORMAT_IEEE_FLOAT, BitDepth, Header};

const FPS: f64 = 59.727500569606;

/// Arguments for the `render` subcommand
#[derive(Args)]
#[clap(author, version, about = "Render a song to an audio file", long_about = None)]
pub struct RenderArgs {
    /// The path to the ROM to use
    #[clap(short, long)]
    rom: PathBuf,

    /// The path to the sav to use
    sav: PathBuf,

    /// The duration of the render, e.g. "3m 10s"
    #[clap(short, long, default_value = "10s")]
    duration: String,
}

/// Render LSDJ .sav and .lsdsng files, or even entire directories for their contents
pub fn render(args: RenderArgs) -> Result<()> {
    let mut gameboy = GameBoy::new(args.rom, None, GameboyConfig { is_dmg: true })
        .context("Could not boot up ROM")?;

    // Run the clock for a little while to skip some weird start-up blip in the audio
    for _ in 0..secs_to_frames(0.01) {
        gameboy.clock_for_frame();
        gameboy.audio_buffers();
    }

    // Press start to start playing the song
    gameboy.press_joypad(JoypadButton::Start);

    // Render the song!
    let mut audio = AudioBuffers {
        all: Vec::with_capacity(0),
        pulse1: Vec::with_capacity(0),
        pulse2: Vec::with_capacity(0),
        wave: Vec::with_capacity(0),
        noise: Vec::with_capacity(0),
    };

    let duration = parse_duration(&args.duration)
        .context("Invalid duration string")?
        .as_secs_f64();

    for _ in 0..secs_to_frames(duration) {
        gameboy.clock_for_frame();
        merge_audio_buffers(&gameboy.audio_buffers(), &mut audio);
    }

    write_channel("/Users/stijn/Desktop/SRPP/audio/srpp_all.wav", audio.all)
        .context("Could not write all")?;

    write_channel(
        "/Users/stijn/Desktop/SRPP/audio/srpp_pulse1.wav",
        audio.pulse1,
    )
    .context("Could not write pulse1")?;

    write_channel(
        "/Users/stijn/Desktop/SRPP/audio/srpp_pulse2.wav",
        audio.pulse2,
    )
    .context("Could not write pulse2")?;

    write_channel("/Users/stijn/Desktop/SRPP/audio/srpp_wave.wav", audio.wave)
        .context("Could not write wave")?;

    write_channel(
        "/Users/stijn/Desktop/SRPP/audio/srpp_noise.wav",
        audio.noise,
    )
    .context("Could not write noise")?;

    Ok(())
}

fn secs_to_frames(secs: f64) -> usize {
    (secs * FPS).ceil() as usize
}

fn merge_audio_buffers(source: &AudioBuffers, target: &mut AudioBuffers) {
    target.all.extend_from_slice(&source.all);
    target.pulse1.extend_from_slice(&source.pulse1);
    target.pulse2.extend_from_slice(&source.pulse2);
    target.wave.extend_from_slice(&source.wave);
    target.noise.extend_from_slice(&source.noise);
}

fn write_channel<P>(path: P, audio: Vec<f32>) -> Result<()>
where
    P: AsRef<Path>,
{
    let path = path.as_ref();

    if check_for_overwrite(path)? {
        let mut writer = File::create(&path).context("Could not create output file")?;

        wav::write(
            Header::new(WAV_FORMAT_IEEE_FLOAT, 2, 44100, 32),
            &BitDepth::ThirtyTwoFloat(audio),
            &mut writer,
        )
        .context("Could not write to output file")?;

        println!("Wrote {}", path.to_string_lossy());
    }

    Ok(())
}
