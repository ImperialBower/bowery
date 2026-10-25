//! Every box of every layout, before anything is drawn: none overlaps
//! another, none leaves the canvas (D6, D7 at the geometry level).
#![allow(non_snake_case)]

use bowery::config::Layout;
use bowery::draw::geometry::{Plan, Rect};
use proptest::prelude::*;

/// What the tests look for in a plan, and the layouts to try.
struct Fixture;

impl Fixture {
    fn overlapping(plan: &Plan) -> Option<(Rect, Rect)> {
        let boxes = plan.boxes();
        for (index, first) in boxes.iter().enumerate() {
            for second in &boxes[index + 1..] {
                if first.intersects(second) {
                    return Some((*first, *second));
                }
            }
        }
        None
    }

    fn outside(plan: &Plan) -> Option<Rect> {
        plan.boxes()
            .into_iter()
            .find(|rect| rect.x + rect.w > plan.width || rect.y + rect.h > plan.height)
    }

    fn layouts() -> impl Strategy<Value = Layout> {
        prop_oneof![
            Just(Layout::Ring),
            Just(Layout::Spotlight),
            Just(Layout::Ledger)
        ]
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2_000))]

    #[test]
    fn geometry__boxes_disjoint(
        layout in Fixture::layouts(),
        live in prop::collection::vec(any::<bool>(), 2..=10),
        folded in prop::collection::vec("[A-Za-z ()]{1,21}", 0..=10),
        outs in prop::option::of(0usize..=44),
        width in 320u32..=1_400,
    ) {
        let plan = Plan::new(layout, &live, &folded, outs, width);

        prop_assert_eq!(None, Fixture::overlapping(&plan), "{:?} {} seats at {}", plan.layout, live.len(), width);
        prop_assert_eq!(None, Fixture::outside(&plan), "{:?} {} seats at {}", plan.layout, live.len(), width);
    }
}

#[test]
fn geometry__every_seat_has_a_plate_but_folded_spotlight_seats() {
    let live = [true, false, true, false, true];
    for layout in [Layout::Ring, Layout::Ledger] {
        let plan = Plan::new(layout, &live, &[], None, 680);
        assert!(
            plan.seats
                .iter()
                .all(|seat| seat.plate.is_some() && seat.bet.is_some()),
            "{layout}"
        );
    }
    let spotlight = Plan::new(
        Layout::Spotlight,
        &live,
        &["Seat 1".to_string(), "Seat 3".to_string()],
        None,
        680,
    );
    let plates: Vec<bool> = spotlight
        .seats
        .iter()
        .map(|seat| seat.plate.is_some())
        .collect();
    assert_eq!(live.to_vec(), plates);
    assert!(spotlight.strip.is_some());
}

#[test]
fn geometry__no_bar_without_outs() {
    for layout in [Layout::Ring, Layout::Spotlight, Layout::Ledger] {
        assert_eq!(
            None,
            Plan::new(layout, &[true, true], &[], None, 680).outs,
            "{layout}"
        );
    }
}

#[test]
fn geometry__the_bar_is_a_box_when_outs_are_given() {
    for layout in [Layout::Ring, Layout::Spotlight, Layout::Ledger] {
        let plan = Plan::new(layout, &[true, true], &[], Some(5), 680);
        let bar = plan.outs.expect("a bar");
        assert!(plan.boxes().contains(&bar), "{layout}");
    }
}

#[test]
fn geometry__ring_holds_ten_seats_in_the_book_column() {
    let plan = Plan::new(Layout::Ring, &[true; 10], &[], None, 680);

    assert_eq!(Layout::Ring, plan.layout);
    assert!((280..=320).contains(&plan.height), "{}", plan.height);
}

#[test]
fn geometry__ring_falls_back_to_ledger_below_520() {
    assert_eq!(
        Layout::Ring,
        Plan::new(Layout::Ring, &[true; 8], &[], None, 520).layout
    );
    assert_eq!(
        Layout::Ledger,
        Plan::new(Layout::Ring, &[true; 2], &[], None, 519).layout
    );
}

#[test]
fn geometry__ring_falls_back_when_the_seats_do_not_fit() {
    assert_eq!(
        Layout::Ledger,
        Plan::new(Layout::Ring, &[true; 10], &[], None, 520).layout
    );
}

#[test]
fn geometry__narrow_widths_draw_at_320() {
    let plan = Plan::new(Layout::Ledger, &[true; 4], &[], None, 200);

    assert_eq!(320, plan.width);
}

#[test]
fn rect__intersects_is_strict() {
    let first = Rect {
        x: 0,
        y: 0,
        w: 10,
        h: 10,
    };
    let touching = Rect {
        x: 10,
        y: 0,
        w: 10,
        h: 10,
    };
    let overlapping = Rect {
        x: 9,
        y: 9,
        w: 10,
        h: 10,
    };

    assert!(!first.intersects(&touching));
    assert!(first.intersects(&overlapping));
}
