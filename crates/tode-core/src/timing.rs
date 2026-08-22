use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageTiming {
    pub at: i64,
    pub origin: i64,
    #[serde(rename = "responseEnd")]
    pub response_end: i64,
    #[serde(rename = "loadEnd")]
    pub load_end: i64,
    #[serde(rename = "domInteractive")]
    pub dom_interactive: i64,
    pub marks: BTreeMap<String, i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LaunchTiming {
    #[serde(rename = "spawnedAt")]
    pub spawned_at: i64,
    pub stages: Vec<(String, i64)>,
}

const STAGES: [(&str, &str); 6] = [
    ("renderer started", "code/didStartRenderer"),
    ("workbench script loaded", "code/didLoadWorkbenchMain"),
    ("workbench starting", "code/willStartWorkbench"),
    ("editors restored", "code/didRestoreEditors"),
    ("workbench ready", "code/didStartWorkbench"),
    ("settled", "code/LifecyclePhase/Eventually"),
];

pub fn format_timing(page: &PageTiming, launch: Option<&LaunchTiming>, now_ms: i64) -> String {
    let total = page
        .marks
        .get("code/didStartWorkbench")
        .copied()
        .unwrap_or(0)
        .max(page.load_end)
        .max(1);
    let seconds = ((now_ms - page.at) as f64 / 1000.0).round() as i64;
    let mut rows = Vec::new();
    if let Some(launch) = launch {
        let previous = launch.stages.last().map_or(0, |stage| stage.1);
        rows.extend(
            launch
                .stages
                .iter()
                .map(|(label, milliseconds)| (format!("tode: {label}"), milliseconds - previous)),
        );
        let before_navigation = page.origin - launch.spawned_at;
        if before_navigation >= 0 {
            rows.push(("browser start to navigation".into(), before_navigation));
        }
    }
    rows.push(("document arrived".into(), page.response_end));
    rows.push(("dom interactive".into(), page.dom_interactive));
    for (label, mark) in STAGES {
        if let Some(milliseconds) = page.marks.get(mark) {
            rows.push((label.into(), *milliseconds));
        }
    }
    let mut output = format!("page load, {seconds}s ago\n\n");
    for (label, milliseconds) in rows {
        let blocks = ((milliseconds as f64 / total as f64) * 34.0)
            .round()
            .max(1.0) as usize;
        output.push_str(&format!(
            "  {label:<24} {milliseconds:>5}ms  {}\n",
            "█".repeat(blocks)
        ));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page() -> PageTiming {
        PageTiming {
            at: 10_000,
            origin: 10_100,
            response_end: 20,
            load_end: 200,
            dom_interactive: 80,
            marks: BTreeMap::from([
                ("code/didStartRenderer".into(), 100),
                ("code/didStartWorkbench".into(), 180),
            ]),
        }
    }

    #[test]
    fn formats_page_marks_and_bars() {
        let output = format_timing(&page(), None, 12_000);
        assert!(output.starts_with("page load, 2s ago\n\n"));
        assert!(output.contains("document arrived"));
        assert!(output.contains("dom interactive"));
        assert!(output.contains("renderer started"));
        assert!(output.contains("workbench ready"));
        assert!(output.contains('█'));
    }

    #[test]
    fn includes_launch_and_navigation_rows() {
        let output = format_timing(
            &page(),
            Some(&LaunchTiming {
                spawned_at: 10_050,
                stages: vec![("runtime".into(), 10), ("profile".into(), 20)],
            }),
            12_000,
        );
        assert!(output.contains("tode: runtime"));
        assert!(output.contains("browser start to navigation"));
    }

    #[test]
    fn total_never_produces_empty_bar() {
        let mut page = page();
        page.load_end = 0;
        page.marks.clear();
        let output = format_timing(&page, None, page.at);
        assert!(output.lines().any(|line| line.contains("█")));
    }
}
