//! Pomodoro state machine. Pure logic: time is always passed in, never read.

use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Focus,
    ShortBreak,
    LongBreak,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Durations {
    pub focus: Duration,
    pub short_break: Duration,
    pub long_break: Duration,
    /// Focus sessions to complete before a long break.
    pub sessions_per_long: u32,
}

impl Default for Durations {
    fn default() -> Self {
        Self {
            focus: Duration::from_secs(25 * 60),
            short_break: Duration::from_secs(5 * 60),
            long_break: Duration::from_secs(15 * 60),
            sessions_per_long: 4,
        }
    }
}

#[derive(Debug)]
pub struct Timer {
    phase: Phase,
    durations: Durations,
    /// Focus sessions completed in the current cycle (reset after a long break).
    completed_focus: u32,
    /// Remaining time while paused.
    paused_remaining: Duration,
    /// Deadline while running. Remaining time is derived from it, so it never drifts.
    ends_at: Option<Instant>,
}

impl Timer {
    pub fn new(durations: Durations) -> Self {
        Self {
            phase: Phase::Focus,
            durations,
            completed_focus: 0,
            paused_remaining: durations.focus,
            ends_at: None,
        }
    }

    pub fn phase(&self) -> Phase {
        self.phase
    }

    pub fn completed_focus(&self) -> u32 {
        self.completed_focus
    }

    pub fn is_running(&self) -> bool {
        self.ends_at.is_some()
    }

    pub fn phase_length(&self) -> Duration {
        match self.phase {
            Phase::Focus => self.durations.focus,
            Phase::ShortBreak => self.durations.short_break,
            Phase::LongBreak => self.durations.long_break,
        }
    }

    pub fn remaining(&self, now: Instant) -> Duration {
        match self.ends_at {
            Some(end) => end.saturating_duration_since(now),
            None => self.paused_remaining,
        }
    }

    /// Fraction of the current phase still left, from 1.0 (fresh) to 0.0.
    pub fn fraction_left(&self, now: Instant) -> f32 {
        let total = self.phase_length().as_secs_f32();
        if total <= 0.0 {
            return 0.0;
        }
        (self.remaining(now).as_secs_f32() / total).clamp(0.0, 1.0)
    }

    pub fn toggle(&mut self, now: Instant) {
        match self.ends_at.take() {
            Some(end) => self.paused_remaining = end.saturating_duration_since(now),
            None => self.ends_at = Some(now + self.paused_remaining),
        }
    }

    /// Restart the current phase, paused.
    pub fn reset(&mut self) {
        self.ends_at = None;
        self.paused_remaining = self.phase_length();
    }

    /// Jump to the next phase without counting the current one as completed.
    pub fn skip(&mut self) {
        self.advance(false);
    }

    /// Advances the phase once the deadline has passed. Returns the phase that just finished.
    pub fn poll(&mut self, now: Instant) -> Option<Phase> {
        let end = self.ends_at?;
        if now < end {
            return None;
        }
        let finished = self.phase;
        self.advance(true);
        Some(finished)
    }

    /// Applies new durations. A paused timer restarts its phase with the new length;
    /// a running one keeps its current deadline.
    pub fn set_durations(&mut self, durations: Durations) {
        self.durations = durations;
        self.completed_focus = self.completed_focus.min(durations.sessions_per_long);
        if !self.is_running() {
            self.reset();
        }
    }

    fn advance(&mut self, completed: bool) {
        self.phase = match self.phase {
            Phase::Focus => {
                if completed {
                    self.completed_focus += 1;
                }
                if self.completed_focus >= self.durations.sessions_per_long {
                    Phase::LongBreak
                } else {
                    Phase::ShortBreak
                }
            }
            Phase::ShortBreak => Phase::Focus,
            Phase::LongBreak => {
                self.completed_focus = 0;
                Phase::Focus
            }
        };
        self.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn short() -> Durations {
        Durations {
            focus: Duration::from_secs(10),
            short_break: Duration::from_secs(2),
            long_break: Duration::from_secs(5),
            sessions_per_long: 2,
        }
    }

    /// Runs the current phase to completion and returns the finished phase.
    fn finish(t: &mut Timer, now: &mut Instant) -> Phase {
        t.toggle(*now);
        *now += t.phase_length();
        t.poll(*now).expect("phase should finish")
    }

    #[test]
    fn starts_paused_in_focus() {
        let t = Timer::new(short());
        let now = Instant::now();
        assert_eq!(t.phase(), Phase::Focus);
        assert!(!t.is_running());
        assert_eq!(t.remaining(now), Duration::from_secs(10));
        assert_eq!(t.fraction_left(now), 1.0);
    }

    #[test]
    fn counts_down_and_pauses() {
        let mut t = Timer::new(short());
        let now = Instant::now();
        t.toggle(now);
        assert_eq!(t.remaining(now + Duration::from_secs(4)), Duration::from_secs(6));
        t.toggle(now + Duration::from_secs(4));
        // Paused time does not advance.
        assert_eq!(t.remaining(now + Duration::from_secs(100)), Duration::from_secs(6));
        assert!((t.fraction_left(now) - 0.6).abs() < 1e-6);
        assert_eq!(t.poll(now + Duration::from_secs(100)), None);
    }

    #[test]
    fn cycles_through_phases_with_long_break() {
        let mut t = Timer::new(short());
        let mut now = Instant::now();
        assert_eq!(finish(&mut t, &mut now), Phase::Focus);
        assert_eq!(t.phase(), Phase::ShortBreak);
        assert_eq!(t.completed_focus(), 1);
        assert!(!t.is_running(), "auto-pauses at phase end");

        assert_eq!(finish(&mut t, &mut now), Phase::ShortBreak);
        assert_eq!(t.phase(), Phase::Focus);

        assert_eq!(finish(&mut t, &mut now), Phase::Focus);
        assert_eq!(t.phase(), Phase::LongBreak);
        assert_eq!(t.completed_focus(), 2);
        assert_eq!(t.remaining(now), Duration::from_secs(5));

        assert_eq!(finish(&mut t, &mut now), Phase::LongBreak);
        assert_eq!(t.phase(), Phase::Focus);
        assert_eq!(t.completed_focus(), 0);
    }

    #[test]
    fn poll_before_deadline_does_nothing() {
        let mut t = Timer::new(short());
        let now = Instant::now();
        t.toggle(now);
        assert_eq!(t.poll(now + Duration::from_secs(9)), None);
        assert_eq!(t.poll(now + Duration::from_secs(10)), Some(Phase::Focus));
    }

    #[test]
    fn reset_restores_full_phase() {
        let mut t = Timer::new(short());
        let now = Instant::now();
        t.toggle(now);
        t.toggle(now + Duration::from_secs(3));
        t.reset();
        assert_eq!(t.remaining(now), Duration::from_secs(10));
        assert_eq!(t.phase(), Phase::Focus);
    }

    #[test]
    fn skip_does_not_count_focus() {
        let mut t = Timer::new(short());
        t.skip();
        assert_eq!(t.phase(), Phase::ShortBreak);
        assert_eq!(t.completed_focus(), 0);
        t.skip();
        assert_eq!(t.phase(), Phase::Focus);
    }

    #[test]
    fn set_durations_resets_paused_but_not_running() {
        let mut t = Timer::new(short());
        let now = Instant::now();
        let mut longer = short();
        longer.focus = Duration::from_secs(60);
        t.set_durations(longer);
        assert_eq!(t.remaining(now), Duration::from_secs(60));

        t.toggle(now);
        longer.focus = Duration::from_secs(120);
        t.set_durations(longer);
        assert_eq!(t.remaining(now), Duration::from_secs(60));
    }
}
