//! The CPU and memory reading checks, beside the module they exercise.

use super::*;

const MEMINFO: &str = "MemTotal:       16300000 kB\n\
                       MemFree:         1000000 kB\n\
                       MemAvailable:    8000000 kB\n\
                       Buffers:          500000 kB\n";

#[test]
fn meminfo_reports_the_used_share() {
    // (16_300_000 - 8_000_000) / 16_300_000 = 50.9% -> 50
    assert_eq!(parse_meminfo(MEMINFO), Some(50));
}

#[test]
fn meminfo_without_available_is_unknown() {
    assert_eq!(parse_meminfo("MemTotal: 100 kB\nMemFree: 50 kB\n"), None);
}

#[test]
fn meminfo_with_a_zero_total_is_unknown() {
    assert_eq!(
        parse_meminfo("MemTotal: 0 kB\nMemAvailable: 0 kB\n"),
        None
    );
}

#[test]
fn stat_parses_idle_and_total() {
    let cpu = parse_stat("cpu  100 0 50 800 50 0 0 0 0 0\ncpu0 10 0 5 80 5 0 0 0 0 0\n")
        .expect("the aggregate cpu line parses");
    assert_eq!(cpu.idle, 850, "idle includes iowait");
    assert_eq!(cpu.total, 1000, "guest time is already in user/nice");
}

#[test]
fn busy_percent_is_the_delta() {
    let previous = Cpu {
        idle: 800,
        total: 1000,
    };
    let current = Cpu {
        idle: 900,
        total: 1200,
    };
    // busy went 200 -> 300 over 200 total jiffies: 50%.
    assert_eq!(busy_percent(previous, current), Some(50));
}

#[test]
fn busy_percent_needs_a_forward_sample() {
    let sample = Cpu { idle: 10, total: 100 };
    assert_eq!(busy_percent(sample, sample), None);
}
