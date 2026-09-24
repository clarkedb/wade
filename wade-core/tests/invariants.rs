//! Property tests over random event sequences (docs/testing.md#invariants).

mod common;

use embedded_graphics::geometry::Point;
use proptest::prelude::*;
use wade_core::character::Expression;
use wade_core::{App, Event, EventKind, Instant, Touch, TouchPhase, View};

fn phase() -> impl Strategy<Value = TouchPhase> {
    prop_oneof![
        Just(TouchPhase::Down),
        Just(TouchPhase::Move),
        Just(TouchPhase::Up)
    ]
}

fn point() -> impl Strategy<Value = Point> {
    (-20i32..340, -20i32..260).prop_map(|(x, y)| Point::new(x, y))
}

fn kind() -> impl Strategy<Value = EventKind> {
    prop_oneof![
        1 => Just(EventKind::Deadline),
        3 => (phase(), point()).prop_map(|(phase, point)| EventKind::Touch(Touch { phase, point })),
    ]
}

/// Events with non-decreasing timestamps, `gap` ms apart.
fn ordered_events() -> impl Strategy<Value = Vec<Event>> {
    prop::collection::vec((0u64..3_000, kind()), 0..60).prop_map(|steps| {
        let mut t = 0;
        steps
            .into_iter()
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

/// Events with arbitrary timestamps, including ones that go backwards and near u64::MAX.
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

/// Deliver `event` as a platform would: first every deadline the app requested before it.
fn deliver(app: &mut App, event: Event) {
    while let Some(d) = app.next_deadline() {
        if d >= event.at {
            break;
        }
        app.handle(Event::deadline(d));
    }
    app.handle(event);
}

proptest! {
    #[test]
    fn next_deadline_is_later_than_last_event(seed: u64, events in ordered_events()) {
        let mut app = App::new(Instant::from_millis(0), seed);
        for event in events {
            deliver(&mut app, event);
            if let Some(d) = app.next_deadline() {
                prop_assert!(d > app.now(), "deadline {d:?} not after now {:?}", app.now());
            }
        }
    }

    #[test]
    fn handle_never_panics(seed: u64, start: u64, events in wild_events()) {
        let mut app = App::new(Instant::from_millis(start), seed);
        for event in events {
            app.handle(event);
            let _ = app.next_deadline();
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
            // Insert spurious Deadline events before this one, between the previous event and it.
            for (idx, offset) in &extras {
                if !events.is_empty() && idx.index(events.len()) == i {
                    let at = Instant::from_millis(event.at.as_millis().saturating_sub(*offset)).max(noisy.now());
                    noisy.handle(Event::deadline(at));
                }
            }
            deliver(&mut plain, *event);
            deliver(&mut noisy, *event);
            prop_assert_eq!(views_discrete(&plain.view()), views_discrete(&noisy.view()));
        }
    }
}

/// The discrete part of a view. Poses may differ by float rounding between paths.
fn views_discrete(view: &View) -> (Expression, bool) {
    match view {
        View::Buddy(b) => (b.expression, b.blinking),
    }
}
