//! Property tests over random event sequences (docs/testing.md#invariants).

use embedded_graphics::geometry::Point;
use embedded_graphics::primitives::Rectangle;
use proptest::prelude::*;
use wade_core::app::{STALL_LIMIT, Screen};
use wade_core::layout;
use wade_core::timer::{CHIME_INTERVAL, CHIMES, TimerPhase, TimerState};
use wade_core::{App, Digit, Effect, Event, EventKind, Instant, Key, Touch, TouchPhase, View};

fn phase() -> impl Strategy<Value = TouchPhase> {
    prop_oneof![
        Just(TouchPhase::Down),
        Just(TouchPhase::Move),
        Just(TouchPhase::Up)
    ]
}

/// Anywhere on or just off the screen, or often the middle of a touch target,
/// so that random sequences press every button and reach every timer state.
fn point() -> impl Strategy<Value = Point> {
    let mut targets = vec![
        layout::WADE_CENTER,
        layout::APPS.center(),
        layout::BACK.center(),
    ];
    targets.extend(layout::TIMER_ROW.iter().map(Rectangle::center));
    prop_oneof![
        1 => (-20i32..340, -20i32..260).prop_map(|(x, y)| Point::new(x, y)),
        2 => prop::sample::select(targets),
    ]
}

fn key() -> impl Strategy<Value = Key> {
    prop_oneof![
        (0u8..10).prop_map(|n| Key::Digit(Digit::new(n).unwrap())),
        Just(Key::Z),
        Just(Key::P)
    ]
}

fn touch(phase: TouchPhase, point: Point) -> EventKind {
    EventKind::Touch(Touch { phase, point })
}

fn kind() -> impl Strategy<Value = EventKind> {
    prop_oneof![
        1 => Just(EventKind::Deadline),
        3 => (phase(), point()).prop_map(|(phase, point)| touch(phase, point)),
        1 => key().prop_map(EventKind::Key),
    ]
}

/// One step of a session: a single event, a tap (a `Down` and an `Up` at one
/// point), or a touch held at one point for up to a minute. Each event comes
/// with the time since the one before it; the first's is the step's own gap.
fn step() -> impl Strategy<Value = Vec<(u64, EventKind)>> {
    let press = |p, hold| {
        vec![
            (0, touch(TouchPhase::Down, p)),
            (hold, touch(TouchPhase::Up, p)),
        ]
    };
    prop_oneof![
        6 => kind().prop_map(|kind| vec![(0, kind)]),
        2 => point().prop_map(move |p| press(p, 50)),
        1 => (point(), 0u64..60_000).prop_map(move |(p, hold)| press(p, hold)),
    ]
}

/// Taps that open the Timer screen, shorten the timer to 1:00, and start it,
/// then pause it if `pause`.
fn one_minute_timer(pause: bool) -> Vec<EventKind> {
    let minus = layout::TIMER_ROW[0].center();
    let center = layout::TIMER_ROW[1].center();
    let mut taps = vec![layout::APPS.center(), minus, minus, minus, minus, center];
    if pause {
        taps.push(center);
    }
    taps.into_iter()
        .flat_map(|p| [touch(TouchPhase::Down, p), touch(TouchPhase::Up, p)])
        .collect()
}

/// Events with non-decreasing timestamps, mostly up to 3 s apart with now and
/// then a gap of up to two minutes. A third of sequences start a 1:00 timer
/// first, so that it finishes and chimes partway through, and a third start
/// one and pause it.
fn ordered_events() -> impl Strategy<Value = Vec<Event>> {
    let gap = prop_oneof![9 => 0u64..3_000, 1 => 0u64..120_000];
    (0u8..3, prop::collection::vec((gap, step()), 0..40)).prop_map(|(prefix, steps)| {
        let prefix = match prefix {
            0 => Vec::new(),
            1 => one_minute_timer(false),
            _ => one_minute_timer(true),
        };
        let steps = prefix
            .into_iter()
            .map(|kind| (100, kind))
            .chain(steps.into_iter().flat_map(|(gap, events)| {
                events
                    .into_iter()
                    .enumerate()
                    .map(move |(i, (after, kind))| (if i == 0 { gap } else { after }, kind))
            }));
        let mut t = 0;
        steps
            .map(|(gap, kind)| {
                t += gap;
                Event {
                    at: Instant::from_millis(t),
                    kind,
                }
            })
            .collect()
    })
}

/// Events with arbitrary timestamps, including ones that go backwards and near `u64::MAX`.
fn wild_events() -> impl Strategy<Value = Vec<Event>> {
    let at = prop_oneof![any::<u64>(), 0u64..10_000, (u64::MAX - 10_000)..=u64::MAX];
    prop::collection::vec((at, kind()), 0..60).prop_map(|v| {
        v.into_iter()
            .map(|(at, kind)| Event {
                at: Instant::from_millis(at),
                kind,
            })
            .collect()
    })
}

/// Deliver `event` as a punctual platform would: first every deadline the app
/// requested before it, each exactly on time.
fn deliver(app: &mut App, event: Event) {
    deliver_deadlines(app, event.at);
    let _ = app.handle(event);
}

/// Deliver every deadline the app requests before `t`, each exactly on time.
fn deliver_deadlines(app: &mut App, t: Instant) {
    while let Some(d) = app.next_deadline() {
        if d >= t {
            break;
        }
        let _ = app.handle(Event::deadline(d));
    }
}

/// Checks `next_deadline` against the documented contract.
fn assert_deadline_later(app: &App) -> Result<(), TestCaseError> {
    if let Some(d) = app.next_deadline() {
        prop_assert!(
            d > app.now(),
            "deadline {d:?} not after now {:?}",
            app.now()
        );
    }
    Ok(())
}

/// Handle `event` on a punctual platform, checking the chimes against the
/// schedule of the timer's current completion: when it finished, and how
/// many chimes have rung since.
fn handle_chimes(
    app: &mut App,
    event: Event,
    schedule: &mut Option<(Instant, u8)>,
) -> Result<(), TestCaseError> {
    let before = app.timer_state();
    let out = app.handle(event);
    let rang = out.effects.iter().filter(|&&e| e == Effect::Chime).count();
    prop_assert!(rang <= 1, "{} chimes at {:?}", rang, event.at);
    if let TimerState::Running { ends_at, .. } = before
        && ends_at <= event.at
    {
        prop_assert_eq!(event.at, ends_at, "finished late");
        prop_assert_eq!(rang, 1, "no chime on finishing");
        *schedule = Some((ends_at, 1));
    } else if let Some((since, rung)) = schedule.as_mut() {
        let next = *since + CHIME_INTERVAL * u32::from(*rung);
        if *rung < CHIMES && event.at >= next {
            prop_assert_eq!(event.at, next, "chime {} rang late", *rung);
            prop_assert_eq!(rang, 1, "chime {} is missing", *rung);
            *rung += 1;
        } else {
            prop_assert_eq!(rang, 0, "an extra chime at {:?}", event.at);
        }
    } else {
        prop_assert_eq!(rang, 0, "a chime at {:?} with no finished timer", event.at);
    }
    if !matches!(app.timer_state(), TimerState::Done { .. }) {
        *schedule = None;
    }
    Ok(())
}

/// Handle `event`, checking the redraw contract: when `redraw` is false, the
/// view must still be the one last drawn.
fn handle_checked(app: &mut App, drawn: &mut View, event: Event) -> Result<(), TestCaseError> {
    if app.handle(event).redraw {
        *drawn = app.view();
    } else {
        prop_assert_eq!(
            app.view(),
            *drawn,
            "view changed at {:?} without redraw",
            event.at
        );
    }
    Ok(())
}

/// A single `Deadline` just short of `STALL_LIMIT` late replays the gap exactly.
#[test]
fn a_stall_within_the_limit_does_not_change_the_view() {
    let tap = Instant::from_millis(1_000);
    let late = Instant::from_millis(1_000) + STALL_LIMIT;
    let mut punctual = App::new(Instant::from_millis(0), 7);
    let mut stalled = App::new(Instant::from_millis(0), 7);
    for app in [&mut punctual, &mut stalled] {
        deliver(
            app,
            Event::touch(tap, TouchPhase::Down, Point::new(160, 120)),
        );
        deliver(app, Event::touch(tap, TouchPhase::Up, Point::new(160, 120)));
    }
    deliver(&mut punctual, Event::deadline(late));
    let _ = stalled.handle(Event::deadline(late));
    assert_eq!(punctual.view(), stalled.view());
}

proptest! {
    #[test]
    fn next_deadline_is_later_than_last_event(seed: u64, events in ordered_events()) {
        let mut app = App::new(Instant::from_millis(0), seed);
        for event in events {
            deliver(&mut app, event);
            assert_deadline_later(&app)?;
        }
    }

    #[test]
    fn handle_never_panics_and_deadlines_stay_later(seed: u64, start: u64, events in wild_events()) {
        let mut app = App::new(Instant::from_millis(start), seed);
        for event in events {
            let _ = app.handle(event);
            assert_deadline_later(&app)?;
            let _ = app.view();
        }
    }

    #[test]
    fn extra_deadlines_do_not_change_the_view(
        seed: u64,
        events in ordered_events(),
        extras in prop::collection::vec((any::<prop::sample::Index>(), 0u64..3_000), 0..20),
    ) {
        let mut plain = App::new(Instant::from_millis(0), seed);
        let mut noisy = App::new(Instant::from_millis(0), seed);

        for (i, event) in events.iter().enumerate() {
            // Spurious Deadline events between the previous event and this one.
            for (idx, offset) in &extras {
                if idx.index(events.len()) == i {
                    let at = Instant::from_millis(event.at.as_millis().saturating_sub(*offset)).max(noisy.now());
                    let _ = noisy.handle(Event::deadline(at));
                }
            }
            deliver(&mut plain, *event);
            deliver(&mut noisy, *event);
            // The whole view, pose included: animation must depend on elapsed time only.
            prop_assert_eq!(plain.view(), noisy.view());
        }
    }

    #[test]
    fn redraw_is_set_whenever_the_view_changes(
        seed: u64,
        events in ordered_events(),
        punctual in prop::collection::vec(any::<bool>(), 1..100),
    ) {
        let mut app = App::new(Instant::from_millis(0), seed);
        let mut drawn = app.view();
        for (event, &punctual) in events.iter().zip(punctual.iter().cycle()) {
            while let Some(d) = app.next_deadline().filter(|&d| punctual && d < event.at) {
                handle_checked(&mut app, &mut drawn, Event::deadline(d))?;
            }
            handle_checked(&mut app, &mut drawn, *event)?;
        }
    }

    #[test]
    fn a_touch_does_nothing_once_the_screen_changes_or_the_timer_finishes(seed: u64, events in ordered_events()) {
        let mut app = App::new(Instant::from_millis(0), seed);
        // The screen and timer when the current touch went down, and whether
        // the screen has changed or the timer finished since. Finishing
        // changes the buttons under the touch even on the Timer screen.
        let mut touch: Option<(Screen, TimerPhase, bool)> = None;
        let interrupted = |app: &App, screen, phase| {
            app.screen() != screen
                || (phase == TimerPhase::Running && app.timer_state().phase() == TimerPhase::Done)
        };
        for event in events {
            deliver_deadlines(&mut app, event.at);
            if let Some((screen, phase, since)) = touch.as_mut() {
                *since |= interrupted(&app, *screen, *phase);
            }
            match event.kind {
                EventKind::Touch(Touch { phase: TouchPhase::Down, .. }) => {
                    let _ = app.handle(event);
                    touch = Some((app.screen(), app.timer_state().phase(), false));
                }
                EventKind::Touch(Touch { phase, .. }) if touch.is_some() => {
                    // The same instant without the sample: what the app does
                    // if the sample is ignored.
                    let mut ignored = app.clone();
                    let _ = ignored.handle(Event::deadline(event.at));
                    let (screen, timer, since) = touch.expect("a touch in progress");
                    let _ = app.handle(event);
                    if since || interrupted(&ignored, screen, timer) {
                        prop_assert_eq!(
                            app.view(),
                            ignored.view(),
                            "a {:?} at {:?} acted after its touch was interrupted",
                            phase,
                            event.at
                        );
                    }
                    if phase == TouchPhase::Up {
                        touch = None;
                    }
                }
                _ => {
                    let _ = app.handle(event);
                }
            }
        }
    }

    #[test]
    fn chimes_ring_on_schedule_until_dismissed(seed: u64, events in ordered_events()) {
        let mut app = App::new(Instant::from_millis(0), seed);
        let mut schedule = None;
        for event in events {
            while let Some(d) = app.next_deadline().filter(|&d| d < event.at) {
                handle_chimes(&mut app, Event::deadline(d), &mut schedule)?;
            }
            handle_chimes(&mut app, event, &mut schedule)?;
        }
    }

    #[test]
    fn late_deadlines_never_add_chimes(
        seed: u64,
        events in ordered_events(),
        punctual in prop::collection::vec(any::<bool>(), 1..100),
    ) {
        let mut app = App::new(Instant::from_millis(0), seed);
        // Chimes rung since the timer last finished, while it stays Done.
        let mut rung = None;
        let mut handle = |app: &mut App, event: Event| -> Result<(), TestCaseError> {
            let finishing = matches!(app.timer_state(), TimerState::Running { ends_at, .. } if ends_at <= event.at);
            let out = app.handle(event);
            let rang = out.effects.iter().filter(|&&e| e == Effect::Chime).count();
            if finishing {
                prop_assert_eq!(rang, 1, "no chime on finishing");
                rung = Some(1);
            } else if let Some(n) = rung.as_mut() {
                *n += rang;
                prop_assert!(*n <= usize::from(CHIMES), "{} chimes", n);
            } else {
                prop_assert_eq!(rang, 0, "a chime with no finished timer");
            }
            if !matches!(app.timer_state(), TimerState::Done { .. }) {
                rung = None;
            }
            Ok(())
        };
        for (event, &punctual) in events.iter().zip(punctual.iter().cycle()) {
            while let Some(d) = app.next_deadline().filter(|&d| punctual && d < event.at) {
                handle(&mut app, Event::deadline(d))?;
            }
            handle(&mut app, *event)?;
        }
    }

    #[test]
    fn late_deadlines_do_not_change_the_view(
        seed: u64,
        events in ordered_events(),
        punctual in prop::collection::vec(any::<bool>(), 1..100),
    ) {
        // `late` never gets requested deadlines on time before an event marked
        // unpunctual: they are folded into the event itself, as after a stalled
        // platform. Both must end up in the same state.
        let mut on_time = App::new(Instant::from_millis(0), seed);
        let mut late = App::new(Instant::from_millis(0), seed);

        for (event, &punctual) in events.iter().zip(punctual.iter().cycle()) {
            deliver(&mut on_time, *event);
            if punctual {
                deliver(&mut late, *event);
            } else {
                let _ = late.handle(*event);
            }
            prop_assert_eq!(on_time.view(), late.view());
        }
    }
}
