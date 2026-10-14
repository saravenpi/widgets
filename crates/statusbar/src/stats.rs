//! CPU and memory readings for the right-hand end of the bar.
//!
//! Both come from `/proc`, which needs no dependency and is present on every
//! Linux: `meminfo` gives the memory in use, and two `stat` samples a moment
//! apart give the CPU share that was busy between them. Only reading the files
//! is I/O; the arithmetic is pure, so it can be tested without the machine
//! being in any particular state.

use std::fs;

/// Memory in use as a whole percentage, or `None` when `/proc/meminfo` cannot
/// be read or parsed.
pub fn memory_percent() -> Option<u32> {
    parse_meminfo(&fs::read_to_string("/proc/meminfo").ok()?)
}

/// The percentage of memory in use, from the contents of `/proc/meminfo`.
///
/// "In use" is `MemTotal - MemAvailable`, not `MemTotal - MemFree`: the kernel
/// counts reclaimable page cache as free, and available is its own estimate of
/// what a new workload could claim without swapping. A file missing either key,
/// or with a zero total, yields `None` rather than a misleading number.
pub fn parse_meminfo(contents: &str) -> Option<u32> {
    let mut total = None;
    let mut available = None;
    for line in contents.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let Some(value) = value.split_whitespace().next() else {
            continue;
        };
        let Ok(value) = value.parse::<u64>() else {
            continue;
        };
        match key {
            "MemTotal" => total = Some(value),
            "MemAvailable" => available = Some(value),
            _ => {}
        }
    }
    let (total, available) = (total?, available?);
    if total == 0 {
        return None;
    }
    Some((((total - available) * 100) / total) as u32)
}

/// One `/proc/stat` CPU sample, in jiffies.
#[derive(Clone, Copy)]
struct Cpu {
    /// Idle jiffies, including iowait.
    idle: u64,
    /// Jiffies accounted for across all states.
    total: u64,
}

impl Cpu {
    fn read() -> Option<Self> {
        parse_stat(&fs::read_to_string("/proc/stat").ok()?)
    }
}

/// Parse the aggregate `cpu` line of `/proc/stat`.
fn parse_stat(contents: &str) -> Option<Cpu> {
    let mut fields = contents.lines().next()?.split_whitespace();
    if fields.next()? != "cpu" {
        return None;
    }
    let values: Vec<u64> = fields.map(|field| field.parse().ok()).collect::<Option<_>>()?;
    // user nice system idle iowait irq softirq steal guest guest_nice. Guest
    // time is already counted in user/nice, so the total stops at steal.
    let idle = values.get(3)? + values.get(4).unwrap_or(&0);
    let total: u64 = values.iter().take(8).sum();
    Some(Cpu { idle, total })
}

/// CPU usage is a delta between samples, so the sampler keeps the previous one.
#[derive(Default)]
pub struct Sampler {
    previous: Option<Cpu>,
}

impl Sampler {
    pub fn new() -> Self {
        Self::default()
    }

    /// The CPU share that was busy since the previous call, as a whole
    /// percentage. The first call only records a baseline and returns `None`.
    pub fn cpu_percent(&mut self) -> Option<u32> {
        let current = Cpu::read()?;
        let percent = self
            .previous
            .and_then(|previous| busy_percent(previous, current));
        self.previous = Some(current);
        percent
    }
}

/// The percentage of the gap between two samples that was busy.
fn busy_percent(previous: Cpu, current: Cpu) -> Option<u32> {
    let total = current.total.checked_sub(previous.total)?;
    let idle = current.idle.checked_sub(previous.idle)?;
    if total == 0 {
        return None;
    }
    Some(((total.saturating_sub(idle) * 100) / total) as u32)
}

#[cfg(test)]
mod tests;
