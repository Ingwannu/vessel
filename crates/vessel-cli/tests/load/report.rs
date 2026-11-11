//! The report's lines: rates, latencies and memory, in integers (the
//! workspace denies float arithmetic) printed with their decimals.

use std::fmt::Write as _;

use crate::fleet::memory;
use crate::run::Phase;

/// One phase's figures as a line of the report.
pub(crate) fn line(report: &mut String, name: &str, phase: &Phase) {
    let micros = u64::try_from(phase.elapsed.as_micros()).unwrap().max(1);
    let per_second = |count: u64| count.checked_mul(1_000_000).unwrap().checked_div(micros);
    let _written = writeln!(
        report,
        "{name}: {records} records, {mib} MiB in {seconds} s = {rate} records/s, \
         {throughput} MiB/s; latency p50 {p50} ms, p99 {p99} ms, max {max} ms",
        records = phase.records,
        mib = tenths(phase.bytes.checked_mul(10).unwrap().checked_div(1 << 20)),
        seconds = hundredths(micros.checked_div(10_000)),
        rate = per_second(phase.records).unwrap(),
        throughput = hundredths(
            per_second(phase.bytes.checked_mul(100).unwrap())
                .and_then(|rate| rate.checked_div(1 << 20))
        ),
        p50 = hundredths(phase_ms(phase, 500)),
        p99 = hundredths(phase_ms(phase, 990)),
        max = hundredths(phase_ms(phase, 1000)),
    );
}

/// A latency quantile in hundredths of a millisecond.
fn phase_ms(phase: &Phase, permille: usize) -> Option<u64> {
    u64::try_from(phase.latency(permille)).ok()?.checked_div(10)
}

/// `n` tenths as `<whole>.<tenth>`.
fn tenths(n: Option<u64>) -> String {
    let n = n.unwrap_or_default();
    format!(
        "{}.{}",
        n.checked_div(10).unwrap(),
        n.checked_rem(10).unwrap()
    )
}

/// `n` hundredths as `<whole>.<hundredths>`.
fn hundredths(n: Option<u64>) -> String {
    let n = n.unwrap_or_default();
    format!(
        "{}.{:02}",
        n.checked_div(100).unwrap(),
        n.checked_rem(100).unwrap()
    )
}

/// The node's and the UI's resident memory now and at their peak.
pub(crate) fn rss(report: &mut String, when: &str, node: u32, ui: u32) {
    let show = |pid| match memory(pid) {
        Some((now, peak)) => format!(
            "{} MiB (peak {})",
            tenths(now.checked_mul(10).unwrap().checked_div(1024)),
            tenths(peak.checked_mul(10).unwrap().checked_div(1024))
        ),
        None => "not measured on this system".to_owned(),
    };
    let _written = writeln!(
        report,
        "memory, {when}: vesseld {}, vessel ui {}",
        show(node),
        show(ui)
    );
}
