/// Checkbox progress tracking for artifacts.
///
/// Parses `- [ ]` / `- [x]` markers from markdown body
/// and computes completion ratios with ASCII progress bars.
///
/// Result of counting checkboxes in markdown text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CheckboxCount {
    pub total: usize,
    pub completed: usize,
}

/// Progress data for a single artifact.
#[derive(Debug, Clone)]
pub struct ArtifactProgress {
    pub id: String,
    pub title: String,
    pub kind: String,
    pub count: CheckboxCount,
}

impl ArtifactProgress {
    /// Completion ratio as 0.0..1.0.
    pub fn ratio(&self) -> f64 {
        if self.count.total == 0 {
            0.0
        } else {
            self.count.completed as f64 / self.count.total as f64
        }
    }

    /// Completion as integer percentage (0..100).
    pub fn percent(&self) -> u32 {
        (self.ratio() * 100.0).round() as u32
    }

    /// Whether all checkboxes are checked.
    pub fn is_complete(&self) -> bool {
        self.count.total > 0 && self.count.completed == self.count.total
    }

    /// Status label for display.
    pub fn status_label(&self) -> &'static str {
        if self.count.total == 0 {
            "-"
        } else if self.is_complete() {
            "DONE"
        } else {
            "WIP"
        }
    }
}

/// Count checkboxes in markdown text.
///
/// Recognizes GitHub-flavored markdown checkboxes:
/// - `- [ ] unchecked`
/// - `- [x] checked` (case-insensitive x/X)
/// - `* [ ] unchecked` (also with *)
pub fn count_checkboxes(body: &str) -> CheckboxCount {
    let mut total = 0;
    let mut completed = 0;

    for line in body.lines() {
        let trimmed = line.trim_start();
        if is_unchecked(trimmed) {
            total += 1;
        } else if is_checked(trimmed) {
            total += 1;
            completed += 1;
        }
    }

    CheckboxCount { total, completed }
}

/// Check if a trimmed line is an unchecked checkbox.
pub fn is_unchecked(line: &str) -> bool {
    line.starts_with("- [ ] ") || line.starts_with("* [ ] ")
}

/// Check if a trimmed line is a checked checkbox.
pub fn is_checked(line: &str) -> bool {
    line.starts_with("- [x] ")
        || line.starts_with("- [X] ")
        || line.starts_with("* [x] ")
        || line.starts_with("* [X] ")
}

/// Extract the text content after the checkbox marker.
/// Returns None if the line is not a checkbox.
pub fn checkbox_text(line: &str) -> Option<(bool, &str)> {
    let trimmed = line.trim_start();
    if is_checked(trimmed) {
        Some((true, &trimmed[6..]))
    } else if is_unchecked(trimmed) {
        Some((false, &trimmed[6..]))
    } else {
        None
    }
}

/// Render an ASCII progress bar.
///
/// `width` is total character count for the bar (default 24).
/// Ratio is clamped to 0.0..1.0.
pub fn render_bar(ratio: f64, width: usize) -> String {
    let clamped = ratio.clamp(0.0, 1.0);
    let filled = (clamped * width as f64).round() as usize;
    let empty = width.saturating_sub(filled);
    format!("{}{}", "█".repeat(filled), "░".repeat(empty))
}

/// Format a single progress line:
/// `RFC-001  ████████████████░░░░░░░░  4/5   ( 80%)  WIP`
pub fn format_progress_line(progress: &ArtifactProgress, id_width: usize) -> String {
    let bar = render_bar(progress.ratio(), 24);
    format!(
        "{:<id_w$}  {}  {}/{:<4}  ({:>3}%)  {}",
        progress.id,
        bar,
        progress.count.completed,
        progress.count.total,
        progress.percent(),
        progress.status_label(),
        id_w = id_width,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count_checkboxes_empty() {
        let c = count_checkboxes("");
        assert_eq!(
            c,
            CheckboxCount {
                total: 0,
                completed: 0
            }
        );
    }

    #[test]
    fn count_checkboxes_no_checkboxes() {
        let body = "# Title\n\nSome text without checkboxes.\n\n- Regular list item\n";
        let c = count_checkboxes(body);
        assert_eq!(c.total, 0);
        assert_eq!(c.completed, 0);
    }

    #[test]
    fn count_checkboxes_mixed() {
        let body = "## Tasks\n\n- [x] Done task\n- [ ] Pending task\n- [x] Another done\n- [ ] Another pending\n";
        let c = count_checkboxes(body);
        assert_eq!(c.total, 4);
        assert_eq!(c.completed, 2);
    }

    #[test]
    fn count_checkboxes_all_done() {
        let body = "- [x] First\n- [x] Second\n- [x] Third\n";
        let c = count_checkboxes(body);
        assert_eq!(c.total, 3);
        assert_eq!(c.completed, 3);
    }

    #[test]
    fn count_checkboxes_uppercase_x() {
        let body = "- [X] Uppercase check\n- [ ] Still pending\n";
        let c = count_checkboxes(body);
        assert_eq!(c.total, 2);
        assert_eq!(c.completed, 1);
    }

    #[test]
    fn count_checkboxes_star_prefix() {
        let body = "* [x] Star done\n* [ ] Star pending\n";
        let c = count_checkboxes(body);
        assert_eq!(c.total, 2);
        assert_eq!(c.completed, 1);
    }

    #[test]
    fn count_checkboxes_indented() {
        let body = "  - [x] Indented done\n    - [ ] Deep indented\n";
        let c = count_checkboxes(body);
        assert_eq!(c.total, 2);
        assert_eq!(c.completed, 1);
    }

    #[test]
    fn render_bar_zero() {
        assert_eq!(render_bar(0.0, 10), "░░░░░░░░░░");
    }

    #[test]
    fn render_bar_full() {
        assert_eq!(render_bar(1.0, 10), "██████████");
    }

    #[test]
    fn render_bar_half() {
        assert_eq!(render_bar(0.5, 10), "█████░░░░░");
    }

    #[test]
    fn render_bar_clamps_above_one() {
        let bar = render_bar(1.5, 10);
        assert_eq!(bar, "██████████");
    }

    #[test]
    fn render_bar_clamps_negative() {
        let bar = render_bar(-0.5, 10);
        assert_eq!(bar, "░░░░░░░░░░");
    }

    #[test]
    fn render_bar_zero_width() {
        assert_eq!(render_bar(0.5, 0), "");
    }

    #[test]
    fn checkbox_text_checked() {
        assert_eq!(checkbox_text("- [x] Done task"), Some((true, "Done task")));
    }

    #[test]
    fn checkbox_text_unchecked() {
        assert_eq!(checkbox_text("- [ ] Pending"), Some((false, "Pending")));
    }

    #[test]
    fn checkbox_text_not_a_checkbox() {
        assert_eq!(checkbox_text("- Regular item"), None);
    }

    #[test]
    fn checkbox_text_star_checked() {
        assert_eq!(checkbox_text("* [X] Star done"), Some((true, "Star done")));
    }

    #[test]
    fn artifact_progress_ratio() {
        let p = ArtifactProgress {
            id: "RFC-001".into(),
            title: "Test".into(),
            kind: "rfc".into(),
            count: CheckboxCount {
                total: 4,
                completed: 3,
            },
        };
        assert_eq!(p.percent(), 75);
        assert!(!p.is_complete());
        assert_eq!(p.status_label(), "WIP");
    }

    #[test]
    fn artifact_progress_complete() {
        let p = ArtifactProgress {
            id: "PRD-001".into(),
            title: "Done".into(),
            kind: "prd".into(),
            count: CheckboxCount {
                total: 5,
                completed: 5,
            },
        };
        assert_eq!(p.percent(), 100);
        assert!(p.is_complete());
        assert_eq!(p.status_label(), "DONE");
    }

    #[test]
    fn artifact_progress_no_checkboxes() {
        let p = ArtifactProgress {
            id: "ADR-001".into(),
            title: "No tasks".into(),
            kind: "adr".into(),
            count: CheckboxCount {
                total: 0,
                completed: 0,
            },
        };
        assert_eq!(p.percent(), 0);
        assert!(!p.is_complete());
        assert_eq!(p.status_label(), "-");
    }

    #[test]
    fn artifact_progress_zero_completed() {
        let p = ArtifactProgress {
            id: "RFC-002".into(),
            title: "New".into(),
            kind: "rfc".into(),
            count: CheckboxCount {
                total: 5,
                completed: 0,
            },
        };
        assert_eq!(p.percent(), 0);
        assert_eq!(p.status_label(), "WIP");
    }

    #[test]
    fn format_progress_line_basic() {
        let p = ArtifactProgress {
            id: "RFC-001".into(),
            title: "CLI Arch".into(),
            kind: "rfc".into(),
            count: CheckboxCount {
                total: 10,
                completed: 7,
            },
        };
        let line = format_progress_line(&p, 8);
        assert!(line.contains("RFC-001"));
        assert!(line.contains("7/10"));
        assert!(line.contains("70%"));
        assert!(line.contains("█"));
        assert!(line.contains("░"));
        assert!(line.contains("WIP"));
    }
}
