//! The `render` subcommand

use anyhow::{Context, Result};
use clap::Args;
use mizu_core::{GameBoy, GameboyConfig, JoypadButton};
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
}

/// Render LSDJ .sav and .lsdsng files, or even entire directories for their contents
pub fn render(args: RenderArgs) -> Result<()> {
    println!("Rendering with ROM {}", args.rom.to_string_lossy());
    println!("Rendering {}", args.sav.to_string_lossy());

    let mut gameboy = GameBoy::new(args.rom, None, GameboyConfig { is_dmg: true })
        .context("Could not boot up ROM")?;

    // Run the clock for a little while to skip some weird start-up blip in the audio
    let boot_up_duration = (FPS * 0.01).ceil() as usize;
    for _ in 0..boot_up_duration {
        gameboy.clock_for_frame();
        gameboy.audio_buffers();
    }

    // Press start to start playing the song
    gameboy.press_joypad(JoypadButton::Start);

    // Render the song!
    let mut audio = AudioBuffers::default();

    for _ in 0..400 {
        gameboy.clock_for_frame();

        let source = gameboy.audio_buffers();

        audio.all.extend_from_slice(&source.all);
        audio.pulse1.extend_from_slice(&source.pulse1);
        audio.pulse2.extend_from_slice(&source.pulse2);
        audio.wave.extend_from_slice(&source.wave);
        audio.noise.extend_from_slice(&source.noise);
    }

    write_channel("/Users/stijn/Desktop/SRPP/srpp_all.wav", audio.all)
        .context("Could not write all")?;

    write_channel("/Users/stijn/Desktop/SRPP/srpp_pulse1.wav", audio.pulse1)
        .context("Could not write pulse1")?;

    write_channel("/Users/stijn/Desktop/SRPP/srpp_pulse2.wav", audio.pulse2)
        .context("Could not write pulse2")?;

    write_channel("/Users/stijn/Desktop/SRPP/srpp_wave.wav", audio.wave)
        .context("Could not write wave")?;

    write_channel("/Users/stijn/Desktop/SRPP/srpp_noise.wav", audio.noise)
        .context("Could not write noise")?;

    Ok(())
}

#[derive(Default)]
struct AudioBuffers {
    pub pulse1: Vec<f32>,
    pub pulse2: Vec<f32>,
    pub wave: Vec<f32>,
    pub noise: Vec<f32>,

    pub all: Vec<f32>,
}

fn write_channel<P>(path: P, audio: Vec<f32>) -> Result<()>
where
    P: AsRef<Path>,
{
    let mut writer = File::create(path).context("Could not create output file")?;

    wav::write(
        Header::new(WAV_FORMAT_IEEE_FLOAT, 2, 44100, 32),
        &BitDepth::ThirtyTwoFloat(audio),
        &mut writer,
    )
    .context("Could not write to output file")?;

    Ok(())
}
