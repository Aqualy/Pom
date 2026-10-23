//! End-of-phase alert: a system notification and a soft synthesized chime.

use std::f32::consts::TAU;
use std::num::NonZero;
use std::thread;
use std::time::Duration;

use crate::timer::Phase;

/// `volume` is 0–100, or `None` for a silent alert.
pub fn fire(finished: Phase, volume: Option<u32>) {
    let (summary, body) = match finished {
        Phase::Focus => ("Focus done", "Time for a break."),
        Phase::ShortBreak | Phase::LongBreak => ("Break over", "Back to focus."),
    };
    // Notification backends can block (D-Bus, macOS), so keep them off the UI thread.
    thread::spawn(move || {
        if let Err(e) = notify_rust::Notification::new()
            .appname("Pom")
            .summary(summary)
            .body(body)
            .show()
        {
            eprintln!("notification failed: {e}");
        }
    });
    if let Some(volume) = volume {
        preview(volume);
    }
}

/// Plays just the chime, e.g. to try out the volume.
pub fn preview(volume: u32) {
    let gain = volume.min(100) as f32 / 100.0;
    thread::spawn(move || play_chime(gain));
}

const SAMPLE_RATE: u32 = 44_100;

/// Two bell-like tones, a fifth apart, each decaying gently. `gain` is 0–1.
fn chime_samples(gain: f32) -> Vec<f32> {
    let notes = [(0.0, 880.0), (0.18, 1318.5)];
    let len = 1.6;
    let n = (len * SAMPLE_RATE as f32) as usize;
    (0..n)
        .map(|i| {
            let t = i as f32 / SAMPLE_RATE as f32;
            notes
                .iter()
                .filter(|(start, _)| t >= *start)
                .map(|(start, freq)| {
                    let dt = t - start;
                    let attack = (dt / 0.008).min(1.0);
                    let decay = (-dt * 3.2).exp();
                    // A quiet octave partial makes it sound less like a test tone.
                    let tone = (TAU * freq * dt).sin() + 0.25 * (TAU * 2.0 * freq * dt).sin();
                    0.22 * gain * attack * decay * tone
                })
                .sum()
        })
        .collect()
}

fn play_chime(gain: f32) {
    let sink = match rodio::DeviceSinkBuilder::open_default_sink() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("audio unavailable: {e}");
            return;
        }
    };
    let samples: Vec<rodio::Sample> = chime_samples(gain).into_iter().map(|s| s as rodio::Sample).collect();
    let duration = Duration::from_secs_f32(samples.len() as f32 / SAMPLE_RATE as f32);
    let source = rodio::buffer::SamplesBuffer::new(
        NonZero::new(1).unwrap(),
        NonZero::new(SAMPLE_RATE).unwrap(),
        samples,
    );
    sink.mixer().add(source);
    // Playback stops when the sink is dropped.
    thread::sleep(duration + Duration::from_millis(200));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chime_stays_within_range() {
        let s = chime_samples(1.0);
        assert!(!s.is_empty());
        assert!(s.iter().all(|v| v.abs() < 1.0));
        assert!(chime_samples(0.0).iter().all(|v| *v == 0.0));
    }
}
