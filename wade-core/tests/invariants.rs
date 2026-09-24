//! Property tests over random event sequences (docs/testing.md#invariants).

use embedded_graphics::geometry::Point;
use proptest::prelude::*;
use wade_core::{App, Event, EventKind, Instant, Touch, TouchPhase};

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

/// Events with non-decreasing timestamps, up to 3 s apart.
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

/// Deliver `event` as a punctual platform would: first every deadline the app
/// requested before it, each exactly on time.
fn deliver(app: &mut App, event: Event) {
    while let Some(d) = app.next_deadline() {
        if d >= event.at {
            break;
        }
        let _ = app.handle(Event::deadline(d));
    }
    let _ = app.handle(event);
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
    fn late_deadlines_do_not_change_the_view(
        seed: u64,
        events in ordered_events(),
        punctual in prop::collection::vec(any::<bool>(), 60),
    ) {
        // `late` never gets requested deadlines on time before an event marked
        // unpunctual: they are folded into the event itself, as after a stalled
        // platform. Both must end up in the same state.
        let mut on_time = App::new(Instant::from_millis(0), seed);
        let mut late = App::new(Instant::from_millis(0), seed);

        for (event, &punctual) in events.iter().zip(&punctual) {
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
