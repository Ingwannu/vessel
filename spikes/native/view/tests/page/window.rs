//! The page's `data/window.test.ts`.

use vessel_app_view::timeline::{
    Align, INSET, Span, covers, guess, off_by, offsets, scroll_for, span,
};

fn even(count: usize, height: u32) -> Vec<u32> {
    offsets(count, |_| None, height)
}

#[test]
fn guesses_unmeasured_heights_from_the_measured_ones() {
    assert_eq!(guess([], 40), 40);
    assert_eq!(guess([10, 30], 40), 20);
    // a and c are guessed at b's 50.
    let measured = [None, Some(50), None];
    let starts = offsets(3, |row| measured[row], guess([50], 40));
    assert_eq!(starts, [0, 50, 100, 150]);
}

#[test]
fn draws_the_items_in_view_and_a_margin_and_keeps_the_rests_space() {
    let starts = even(1000, 20);
    assert_eq!(starts.last(), Some(&20_000));
    // Rows 100 to 129 are in a 600 px view at 2,000 px; 200 px of margin
    // adds 10 rows each way.
    let window = span(&starts, 2000, 600, 200);
    assert_eq!(
        window,
        Span {
            start: 90,
            end: 140,
            before: 1800,
            after: 17_200
        }
    );
    assert_eq!(window.before + 50 * 20 + window.after, 20_000);
}

#[test]
fn stays_inside_the_list_at_its_ends() {
    let starts = even(1000, 20);
    assert_eq!(
        span(&starts, 0, 600, 200),
        Span {
            start: 0,
            end: 40,
            before: 0,
            after: 19_200
        }
    );
    let end = span(&starts, 19_400, 600, 200);
    assert_eq!((end.end, end.after), (1000, 0));
    // Scrolled past the end (the list just shrank): the last item.
    let past = span(&starts, 50_000, 600, 0);
    assert_eq!((past.start, past.end), (999, 1000));
    assert_eq!(
        span(&[0], 0, 600, 200),
        Span {
            start: 0,
            end: 0,
            before: 0,
            after: 0
        }
    );
}

#[test]
fn follows_measured_heights() {
    let heights: Vec<Option<u32>> = (0..1000).map(|row| (row < 10).then_some(100)).collect();
    let starts = offsets(1000, |row| heights[row], guess([100; 10], 20));
    // Ten rows of 100, the rest guessed at their mean, 100.
    assert_eq!(starts[10], 1000);
    let window = span(&starts, 950, 100, 0);
    assert_eq!((window.start, window.end), (9, 11));
}

#[test]
fn puts_its_top_under_the_views_top_or_its_middle_in_the_middle() {
    let starts = even(1000, 20);
    // Item 500 starts at 10,000 px and ends at 10,020.
    assert_eq!(scroll_for(&starts, 500, 600, Align::Start), 10_000 - INSET);
    assert_eq!(scroll_for(&starts, 500, 600, Align::Center), 10_010 - 300);
    // Near the top there is nothing above to scroll past.
    assert_eq!(scroll_for(&starts, 0, 600, Align::Center), 0);
    assert_eq!(scroll_for(&starts, 0, 600, Align::Start), 0);
}

#[test]
fn corrects_by_what_the_drawn_box_says() {
    let inset = i64::from(INSET);
    // Drawn 40 px lower than asked for: scroll 40 further.
    assert_eq!(off_by(inset + 40, 20, 600, Align::Start), 40);
    assert_eq!(off_by(290, 20, 600, Align::Center), 0);
    assert_eq!(off_by(250, 20, 600, Align::Center), -40);
    // A box taller than the view is held by its top.
    assert_eq!(off_by(inset, 900, 600, Align::Center), 0);
}

#[test]
fn knows_when_the_drawn_items_leave_part_of_the_view_empty() {
    let starts = even(1000, 20);
    let drawn = span(&starts, 10_000, 600, 200);
    assert!(covers(&starts, drawn, 10_000, 600));
    // A correction of more than the margin uncovers the view.
    assert!(!covers(&starts, drawn, 10_300, 600));
    assert!(!covers(&starts, drawn, 9_700, 600));
    assert!(covers(
        &[0],
        Span {
            start: 0,
            end: 0,
            before: 0,
            after: 0
        },
        0,
        600
    ));
}

#[test]
fn follows_the_end_within_48_pixels_of_it() {
    use vessel_app_view::timeline::at_end;
    assert!(at_end(9_352, 600, 10_000));
    assert!(!at_end(9_351, 600, 10_000));
    assert!(at_end(0, 600, 300), "content shorter than the view");
}
