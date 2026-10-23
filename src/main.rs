// No console window in release builds on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod alert;
mod config;
mod geometry;
mod timer;

use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use slint::{Model, ModelRc, SharedString, VecModel};

use config::Config;
use timer::{Phase, Timer};

slint::include_modules!();

/// Mutable app state shared by the UI callbacks, all on the UI thread.
struct App {
    config: Config,
    timer: Timer,
    tomato: geometry::Tomato,
    time_chars: Rc<VecModel<SharedString>>,
}

impl App {
    fn refresh(&self, ui: &AppWindow) {
        let now = Instant::now();
        let t = &self.timer;
        ui.set_phase(match t.phase() {
            Phase::Focus => 0,
            Phase::ShortBreak => 1,
            Phase::LongBreak => 2,
        });
        ui.set_running(t.is_running());
        ui.set_completed(t.completed_focus() as i32);
        ui.set_body_path(self.tomato.wedge(t.fraction_left(now)).into());

        let text = format_remaining(t.remaining(now));
        let unchanged = self.time_chars.row_count() == text.chars().count()
            && self.time_chars.iter().zip(text.chars()).all(|(a, b)| a.as_str() == b.encode_utf8(&mut [0; 4]));
        if !unchanged {
            self.time_chars.set_vec(text.chars().map(|c| SharedString::from(c.to_string())).collect::<Vec<_>>());
        }
    }

    fn load_settings_into(&self, ui: &AppWindow) {
        let c = &self.config;
        ui.set_focus_min(c.focus_min as i32);
        ui.set_short_break_min(c.short_break_min as i32);
        ui.set_long_break_min(c.long_break_min as i32);
        ui.set_sessions(c.sessions_per_long as i32);
        ui.set_sound(c.sound);
        ui.set_volume(c.volume as i32);
        ui.set_pinned(c.always_on_top);
    }

    fn read_settings_from(&mut self, ui: &AppWindow) {
        let to_u32 = |v: i32| v.max(0) as u32;
        self.config = Config {
            focus_min: to_u32(ui.get_focus_min()),
            short_break_min: to_u32(ui.get_short_break_min()),
            long_break_min: to_u32(ui.get_long_break_min()),
            sessions_per_long: to_u32(ui.get_sessions()),
            sound: ui.get_sound(),
            volume: to_u32(ui.get_volume()),
            always_on_top: ui.get_pinned(),
        }
        .clamped();
    }
}

/// `mm:ss`, rounding up so the display reads 25:00 at the start and 00:01 just before the end.
fn format_remaining(d: Duration) -> String {
    let secs = d.as_millis().div_ceil(1000);
    format!("{:02}:{:02}", secs / 60, secs % 60)
}

fn main() -> Result<(), slint::PlatformError> {
    let ui = AppWindow::new()?;

    let config = Config::load();
    let app = Rc::new(RefCell::new(App {
        timer: Timer::new(config.durations()),
        config,
        tomato: geometry::Tomato::load(),
        time_chars: Rc::new(VecModel::default()),
    }));

    {
        let a = app.borrow();
        let (x, y, size) = a.tomato.viewbox;
        ui.set_art_x(x);
        ui.set_art_y(y);
        ui.set_art_size(size);
        ui.set_ghost_path(a.tomato.body.as_str().into());
        ui.set_crown_path(a.tomato.crown.as_str().into());
        ui.set_time_chars(ModelRc::from(a.time_chars.clone()));
        a.load_settings_into(&ui);
        a.refresh(&ui);
    }

    // Wires a callback that mutates the app and then redraws.
    let with_app = |f: fn(&mut App, &AppWindow)| {
        let app = app.clone();
        let weak = ui.as_weak();
        move || {
            let Some(ui) = weak.upgrade() else { return };
            let mut a = app.borrow_mut();
            f(&mut a, &ui);
            a.refresh(&ui);
        }
    };

    ui.on_toggle(with_app(|a, _| a.timer.toggle(Instant::now())));
    ui.on_reset(with_app(|a, _| a.timer.reset()));
    ui.on_skip(with_app(|a, _| a.timer.skip()));
    ui.on_settings_done(with_app(|a, ui| {
        a.read_settings_from(ui);
        a.load_settings_into(ui);
        a.timer.set_durations(a.config.durations());
        a.config.save();
    }));
    ui.on_pin_changed(with_app(|a, ui| {
        a.config.always_on_top = ui.get_pinned();
        a.config.save();
    }));
    ui.on_preview_sound({
        let weak = ui.as_weak();
        move || {
            let Some(ui) = weak.upgrade() else { return };
            alert::preview(ui.get_volume().max(0) as u32);
        }
    });
    ui.on_quit(|| {
        let _ = slint::quit_event_loop();
    });

    // The deadline lives in `Timer`, so this tick only drives the display and phase changes.
    let ticker = slint::Timer::default();
    ticker.start(slint::TimerMode::Repeated, Duration::from_millis(100), {
        let app = app.clone();
        let weak = ui.as_weak();
        move || {
            let Some(ui) = weak.upgrade() else { return };
            let mut a = app.borrow_mut();
            if let Some(finished) = a.timer.poll(Instant::now()) {
                alert::fire(finished, a.config.sound.then_some(a.config.volume));
            }
            a.refresh(&ui);
        }
    });

    ui.run()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_remaining_rounding_up() {
        assert_eq!(format_remaining(Duration::from_secs(25 * 60)), "25:00");
        assert_eq!(format_remaining(Duration::from_millis(1_500)), "00:02");
        assert_eq!(format_remaining(Duration::from_millis(1)), "00:01");
        assert_eq!(format_remaining(Duration::ZERO), "00:00");
    }
}
