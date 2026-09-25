//! The countdown timer: `TimerState` and its transitions (docs/ui.md#timer).

use core::fmt;

use crate::time::{Duration, Instant, millis};

/// The shortest duration that can be set.
pub const MIN_SET: Duration = Duration::from_mins(1);
/// The longest duration that can be set.
pub const MAX_SET: Duration = Duration::from_mins(99);
/// How much −1m and +1m change the duration.
pub const STEP: Duration = Duration::from_mins(1);
/// The duration a new timer starts with.
pub const DEFAULT_SET: Duration = Duration::from_mins(5);
/// While Done, the chime repeats this often.
pub const CHIME_INTERVAL: Duration = Duration::from_secs(2);
/// How many chimes play once the timer finishes, counting the first.
pub const CHIMES: u8 = 10;

/// The timer. `set`, the chosen duration, is kept in every state so that Reset
/// and Dismiss return to Ready with it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TimerState {
    Ready {
        set: Duration,
    },
    Running {
        set: Duration,
        ends_at: Instant,
    },
    Paused {
        set: Duration,
        remaining: Duration,
    },
    /// Finished at `since`. `chimes` counts the chimes that have fallen due;
    /// they fall at `since`, `since + CHIME_INTERVAL`, and so on.
    Done {
        set: Duration,
        since: Instant,
        chimes: u8,
    },
}

/// Which state the timer is in, without its times.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TimerPhase {
    Ready,
    Running,
    Paused,
    Done,
}

/// A button in the Timer screen's row.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TimerButton {
    Minus,
    Plus,
    Start,
    Pause,
    Resume,
    Reset,
    Dismiss,
}

/// A button in the row as shown. A disabled one is dimmed and ignores taps.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RowButton {
    pub button: TimerButton,
    pub enabled: bool,
}

/// A chime the timer asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Chime {
    /// The timer has just finished.
    First,
    /// A repeat while the timer stays Done.
    Repeat,
}

/// A time as the Timer screen shows it: whole minutes and seconds, MM:SS.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Digits {
    minutes: u8,
    seconds: u8,
}

impl TimerState {
    /// Ready, with the default duration.
    #[must_use]
    pub const fn new() -> Self {
        TimerState::Ready { set: DEFAULT_SET }
    }

    /// The chosen duration, `set`.
    #[must_use]
    pub const fn duration(&self) -> Duration {
        match *self {
            TimerState::Ready { set }
            | TimerState::Running { set, .. }
            | TimerState::Paused { set, .. }
            | TimerState::Done { set, .. } => set,
        }
    }

    #[must_use]
    pub const fn phase(&self) -> TimerPhase {
        match self {
            TimerState::Ready { .. } => TimerPhase::Ready,
            TimerState::Running { .. } => TimerPhase::Running,
            TimerState::Paused { .. } => TimerPhase::Paused,
            TimerState::Done { .. } => TimerPhase::Done,
        }
    }

    /// The time left at `now`: all of `set` until started, none once Done.
    #[must_use]
    pub const fn remaining(&self, now: Instant) -> Duration {
        match *self {
            TimerState::Ready { set } => set,
            TimerState::Running { ends_at, .. } => ends_at.saturating_since(now),
            TimerState::Paused { remaining, .. } => remaining,
            TimerState::Done { .. } => Duration::ZERO,
        }
    }

    /// The time shown at `now`: the time left, rounded up to whole seconds, so
    /// the last second shows 00:01.
    #[must_use]
    pub fn digits(&self, now: Instant) -> Digits {
        Digits::rounding_up(self.remaining(now))
    }

    /// The next scheduled change: finishing while Running, and the next chime
    /// while Done, until the last. After `advance(now)` it is later than `now`.
    #[must_use]
    pub fn next_transition(&self) -> Option<Instant> {
        match *self {
            TimerState::Running { ends_at, .. } => Some(ends_at),
            TimerState::Done { since, chimes, .. } if chimes < CHIMES => {
                Some(chime_at(since, chimes))
            }
            _ => None,
        }
    }

    /// The next instant after `now` at which the digits change while Running.
    /// Digits change on second boundaries counted back from `ends_at`, at
    /// `ends_at − k × 1 s`, not on whole seconds of the clock.
    #[must_use]
    pub fn next_tick(&self, now: Instant) -> Option<Instant> {
        let TimerState::Running { ends_at, .. } = *self else {
            return None;
        };
        let left = millis(ends_at.saturating_since(now));
        // The digits next change when one second fewer remains than they show.
        // That is less than `left`, so the subtraction cannot underflow.
        let fewer = left.div_ceil(1_000).checked_sub(1)?;
        Some(Instant::from_millis(ends_at.as_millis() - fewer * 1_000))
    }

    /// Apply the transitions due at `now`: finishing, then the chimes while
    /// Done. Chimes that fall due together, after a late wake, ring once.
    #[must_use = "a chime must be played"]
    pub fn advance(&mut self, now: Instant) -> Option<Chime> {
        let mut rang = None;
        if let TimerState::Running { set, ends_at } = *self
            && ends_at <= now
        {
            *self = TimerState::Done {
                set,
                since: ends_at,
                chimes: 1,
            };
            rang = Some(Chime::First);
        }
        if let TimerState::Done { since, chimes, .. } = self {
            while *chimes < CHIMES && chime_at(*since, *chimes) <= now {
                *chimes += 1;
                rang = rang.or(Some(Chime::Repeat));
            }
        }
        rang
    }

    /// The row's left, center, and right buttons. `None` leaves a slot empty.
    #[must_use]
    pub fn row(&self) -> [Option<RowButton>; 3] {
        let on = |button| {
            Some(RowButton {
                button,
                enabled: true,
            })
        };
        match *self {
            TimerState::Ready { set } => [
                Some(RowButton {
                    button: TimerButton::Minus,
                    enabled: shorter(set).is_some(),
                }),
                on(TimerButton::Start),
                Some(RowButton {
                    button: TimerButton::Plus,
                    enabled: longer(set).is_some(),
                }),
            ],
            TimerState::Running { .. } => [None, on(TimerButton::Pause), None],
            TimerState::Paused { .. } => [on(TimerButton::Reset), on(TimerButton::Resume), None],
            TimerState::Done { .. } => [None, on(TimerButton::Dismiss), None],
        }
    }

    /// Press `button` at `now`. Returns false, changing nothing, if the button
    /// is not in the row or is disabled.
    #[must_use = "a true result means the view must be redrawn"]
    pub fn press(&mut self, button: TimerButton, now: Instant) -> bool {
        let next = match (*self, button) {
            (TimerState::Ready { set }, TimerButton::Minus) => {
                shorter(set).map(|set| TimerState::Ready { set })
            }
            (TimerState::Ready { set }, TimerButton::Plus) => {
                longer(set).map(|set| TimerState::Ready { set })
            }
            (TimerState::Ready { set }, TimerButton::Start) => Some(TimerState::Running {
                set,
                ends_at: now + set,
            }),
            // At `ends_at` the timer has finished, whether or not it has been
            // advanced yet; a Pause then would leave nothing to resume.
            (TimerState::Running { set, ends_at }, TimerButton::Pause) if ends_at > now => {
                Some(TimerState::Paused {
                    set,
                    remaining: ends_at.saturating_since(now),
                })
            }
            (TimerState::Paused { set, remaining }, TimerButton::Resume) => {
                Some(TimerState::Running {
                    set,
                    ends_at: now + remaining,
                })
            }
            (TimerState::Paused { set, .. }, TimerButton::Reset)
            | (TimerState::Done { set, .. }, TimerButton::Dismiss) => {
                Some(TimerState::Ready { set })
            }
            _ => None,
        };
        let Some(next) = next else {
            return false;
        };
        *self = next;
        true
    }
}

impl Default for TimerState {
    fn default() -> Self {
        TimerState::new()
    }
}

impl Digits {
    /// `minutes` and `seconds`, clamped to 99 and 59.
    #[must_use]
    pub const fn new(minutes: u8, seconds: u8) -> Self {
        Digits {
            minutes: if minutes > 99 { 99 } else { minutes },
            seconds: if seconds > 59 { 59 } else { seconds },
        }
    }

    /// `d` rounded up to whole seconds, at most 99:59.
    #[must_use]
    pub fn rounding_up(d: Duration) -> Self {
        let seconds = d.as_nanos().div_ceil(1_000_000_000).min(99 * 60 + 59);
        let part = |n: u128| u8::try_from(n).unwrap_or(u8::MAX);
        Digits::new(part(seconds / 60), part(seconds % 60))
    }

    #[must_use]
    pub const fn minutes(self) -> u8 {
        self.minutes
    }

    #[must_use]
    pub const fn seconds(self) -> u8 {
        self.seconds
    }
}

impl fmt::Display for Digits {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02}:{:02}", self.minutes, self.seconds)
    }
}

/// When chime `n` (counting from zero) falls.
fn chime_at(since: Instant, n: u8) -> Instant {
    since + CHIME_INTERVAL.saturating_mul(u32::from(n))
}

/// One step shorter than `set`, unless that is below the shortest.
fn shorter(set: Duration) -> Option<Duration> {
    set.checked_sub(STEP).filter(|&set| set >= MIN_SET)
}

/// One step longer than `set`, unless that is above the longest.
fn longer(set: Duration) -> Option<Duration> {
    set.checked_add(STEP).filter(|&set| set <= MAX_SET)
}

#[cfg(test)]
mod tests {
    use std::string::ToString;

    use super::*;

    fn ms(ms: u64) -> Instant {
        Instant::from_millis(ms)
    }

    fn running(ends_at: u64) -> TimerState {
        TimerState::Running {
            set: DEFAULT_SET,
            ends_at: ms(ends_at),
        }
    }

    /// Advance `timer` through every transition up to `until`, collecting
    /// each chime with its time.
    fn run(timer: &mut TimerState, until: Instant) -> std::vec::Vec<(Instant, Chime)> {
        let mut chimes = std::vec::Vec::new();
        while let Some(t) = timer.next_transition().filter(|&t| t <= until) {
            chimes.extend(timer.advance(t).map(|chime| (t, chime)));
        }
        chimes
    }

    #[test]
    fn starts_ready_at_five_minutes() {
        let timer = TimerState::new();
        assert_eq!(timer, TimerState::Ready { set: DEFAULT_SET });
        assert_eq!(timer.digits(ms(0)).to_string(), "05:00");
    }

    #[test]
    fn start_runs_until_now_plus_set() {
        let mut timer = TimerState::new();
        assert!(timer.press(TimerButton::Start, ms(1_000)));
        assert_eq!(timer, running(301_000));
    }

    #[test]
    fn digits_round_up_to_whole_seconds() {
        let timer = running(300_000);
        assert_eq!(timer.digits(ms(999)).to_string(), "05:00");
        assert_eq!(timer.digits(ms(1_000)).to_string(), "04:59");
        assert_eq!(timer.digits(ms(299_500)).to_string(), "00:01");
        assert_eq!(timer.digits(ms(300_000)).to_string(), "00:00");
        assert_eq!(
            Digits::rounding_up(Duration::from_nanos(1)).to_string(),
            "00:01"
        );
        assert_eq!(
            Digits::rounding_up(Duration::from_hours(3)).to_string(),
            "99:59"
        );
        assert_eq!(Digits::new(250, 250), Digits::new(99, 59));
    }

    #[test]
    fn digits_tick_on_seconds_counted_back_from_the_end() {
        let timer = running(10_250);
        assert_eq!(timer.next_tick(ms(0)), Some(ms(250)));
        assert_eq!(timer.next_tick(ms(250)), Some(ms(1_250)));
        assert_eq!(timer.next_tick(ms(9_300)), Some(ms(10_250)));
        assert_eq!(timer.next_tick(ms(10_250)), None);
        assert_eq!(TimerState::new().next_tick(ms(0)), None);
    }

    #[test]
    fn pause_and_resume_keep_the_time_left() {
        let mut timer = running(300_000);
        assert!(timer.press(TimerButton::Pause, ms(100_400)));
        assert_eq!(
            timer,
            TimerState::Paused {
                set: DEFAULT_SET,
                remaining: Duration::from_millis(199_600),
            }
        );
        assert_eq!(timer.digits(ms(500_000)).to_string(), "03:20");
        assert_eq!(timer.next_transition(), None);
        assert!(timer.press(TimerButton::Resume, ms(500_000)));
        assert_eq!(timer, running(699_600));
    }

    #[test]
    fn pause_does_nothing_once_the_end_is_reached() {
        let mut timer = running(300_000);
        assert!(!timer.press(TimerButton::Pause, ms(300_000)));
        assert_eq!(timer, running(300_000));
    }

    #[test]
    fn reset_and_dismiss_keep_the_duration() {
        let set = Duration::from_mins(3);
        let mut paused = TimerState::Paused {
            set,
            remaining: Duration::from_secs(20),
        };
        assert!(paused.press(TimerButton::Reset, ms(0)));
        assert_eq!(paused, TimerState::Ready { set });
        let mut done = TimerState::Done {
            set,
            since: ms(0),
            chimes: 3,
        };
        assert!(done.press(TimerButton::Dismiss, ms(25_000)));
        assert_eq!(done, TimerState::Ready { set });
    }

    #[test]
    fn minus_and_plus_stop_at_one_and_ninety_nine_minutes() {
        let mut timer = TimerState::Ready { set: MIN_SET };
        assert_eq!(timer.row()[0].map(|b| b.enabled), Some(false));
        assert!(!timer.press(TimerButton::Minus, ms(0)));
        assert_eq!(timer.duration(), MIN_SET);
        assert!(timer.press(TimerButton::Plus, ms(0)));
        assert_eq!(timer.duration(), Duration::from_mins(2));

        let mut timer = TimerState::Ready { set: MAX_SET };
        assert_eq!(timer.row()[2].map(|b| b.enabled), Some(false));
        assert!(!timer.press(TimerButton::Plus, ms(0)));
        assert_eq!(timer.digits(ms(0)).to_string(), "99:00");
        assert!(timer.press(TimerButton::Minus, ms(0)));
        assert_eq!(timer.duration(), Duration::from_mins(98));
    }

    #[test]
    fn buttons_outside_the_row_do_nothing() {
        let mut timer = running(300_000);
        for button in [
            TimerButton::Minus,
            TimerButton::Plus,
            TimerButton::Start,
            TimerButton::Resume,
            TimerButton::Reset,
            TimerButton::Dismiss,
        ] {
            assert!(
                !timer.press(button, ms(0)),
                "{button:?} acted while Running"
            );
        }
        assert_eq!(timer, running(300_000));
    }

    #[test]
    fn each_state_has_its_row() {
        use TimerButton as B;
        let buttons = |timer: TimerState| timer.row().map(|slot| slot.map(|b| b.button));
        assert_eq!(
            buttons(TimerState::new()),
            [Some(B::Minus), Some(B::Start), Some(B::Plus)]
        );
        assert_eq!(buttons(running(1)), [None, Some(B::Pause), None]);
        let paused = TimerState::Paused {
            set: MIN_SET,
            remaining: MIN_SET,
        };
        assert_eq!(buttons(paused), [Some(B::Reset), Some(B::Resume), None]);
        let done = TimerState::Done {
            set: MIN_SET,
            since: ms(0),
            chimes: 1,
        };
        assert_eq!(buttons(done), [None, Some(B::Dismiss), None]);
    }

    #[test]
    fn finishing_chimes_ten_times_an_interval_apart() {
        let mut timer = running(60_000);
        let chimes = run(&mut timer, ms(1_000_000));
        let expected: std::vec::Vec<_> = (0..10u32)
            .map(|k| {
                let chime = if k == 0 { Chime::First } else { Chime::Repeat };
                (ms(60_000) + CHIME_INTERVAL * k, chime)
            })
            .collect();
        assert_eq!(chimes, expected);
        assert_eq!(
            timer,
            TimerState::Done {
                set: DEFAULT_SET,
                since: ms(60_000),
                chimes: CHIMES,
            }
        );
        assert_eq!(timer.next_transition(), None);
    }

    #[test]
    fn a_late_advance_finishes_at_the_end_and_rings_once() {
        let mut timer = running(60_000);
        let late = ms(60_000) + CHIME_INTERVAL * 2 + CHIME_INTERVAL / 2;
        assert_eq!(timer.advance(late), Some(Chime::First));
        assert_eq!(
            timer,
            TimerState::Done {
                set: DEFAULT_SET,
                since: ms(60_000),
                chimes: 3,
            }
        );
        assert_eq!(
            timer.next_transition(),
            Some(ms(60_000) + CHIME_INTERVAL * 3)
        );
        assert_eq!(timer.advance(late), None);
    }

    #[test]
    fn deadlines_for_each_state() {
        assert_eq!(TimerState::new().next_transition(), None);
        assert_eq!(running(5_000).next_transition(), Some(ms(5_000)));
        let paused = TimerState::Paused {
            set: MIN_SET,
            remaining: MIN_SET,
        };
        assert_eq!(paused.next_transition(), None);
        let done = |chimes| TimerState::Done {
            set: MIN_SET,
            since: ms(1_000),
            chimes,
        };
        assert_eq!(done(1).next_transition(), Some(ms(1_000) + CHIME_INTERVAL));
        assert_eq!(
            done(9).next_transition(),
            Some(ms(1_000) + CHIME_INTERVAL * 9)
        );
        assert_eq!(done(10).next_transition(), None);
    }

    #[test]
    fn a_timer_started_at_the_end_of_time_still_finishes() {
        let mut timer = TimerState::new();
        assert!(timer.press(TimerButton::Start, Instant::MAX));
        assert_eq!(timer.next_transition(), Some(Instant::MAX));
        assert_eq!(timer.next_tick(Instant::MAX), None);
        assert_eq!(timer.advance(Instant::MAX), Some(Chime::First));
        assert_eq!(timer.next_transition(), None);
    }
}
