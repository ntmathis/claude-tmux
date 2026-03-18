use crate::session::ClaudeCodeStatus;

/// Detect input field: prompt line (❯) with border directly above it.
/// Returns the index of the prompt line if found. Searches from bottom to find the
/// most recent (current) prompt, not an old one in scrollback.
fn find_input_field_line(lines: &[&str]) -> Option<usize> {
    // Iterate from bottom up to find the most recent prompt
    for (i, line) in lines.iter().enumerate().rev() {
        if line.contains('❯') {
            // Check if line above is a border
            if i > 0 && lines[i - 1].contains('─') {
                return Some(i);
            }
        }
    }
    None
}

/// Check if a line indicates working state (interruptible)
fn is_interrupt_line(line: &str) -> bool {
    // Claude shows "esc to interrupt" or "ctrl+c to interrupt" when working.
    // Just check for "interrupt" - it's only shown in working state.
    line.contains("interrupt")
}

pub fn detect_status(content: &str) -> ClaudeCodeStatus {
    let lines: Vec<&str> = content.lines().collect();

    if let Some(i) = find_input_field_line(&lines) {
        // Search a window: 2 lines above, prompt, and up to 20 lines below.
        // This works for typical desktop layouts.
        let start = i.saturating_sub(2);
        let end = std::cmp::min(i + 20, lines.len().saturating_sub(1));
        for idx in start..=end {
            let line = lines[idx];
            // Match full "interrupt" or truncated "esc to…"
            if line.contains("interrupt") || line.contains("esc t") {
                return ClaudeCodeStatus::Working;
            }
        }
        return ClaudeCodeStatus::Idle;
    }

    // No input field - check for permission prompt
    if content.contains("[y/n]") || content.contains("[Y/n]") {
        return ClaudeCodeStatus::WaitingInput;
    }

    ClaudeCodeStatus::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_working() {
        // Border directly above prompt
        let content = "* (ctrl+c to interrupt)\n─────\n❯ hello";
        assert_eq!(detect_status(content), ClaudeCodeStatus::Working);
    }

    #[test]
    fn test_working_with_esc() {
        // New format: esc to interrupt
        let content = "* (esc to interrupt)\n─────\n❯ hello";
        assert_eq!(detect_status(content), ClaudeCodeStatus::Working);
    }

    #[test]
    fn test_working_with_escape_uppercase() {
        let content = "Ctrl+C to interrupt\n─────\n❯ hello";
        assert_eq!(detect_status(content), ClaudeCodeStatus::Working);
    }

    #[test]
    fn test_working_esc_on_prompt_line() {
        // Interrupt message on same line as prompt
        let content = "─────\n❯ esc to interrupt";
        assert_eq!(detect_status(content), ClaudeCodeStatus::Working);
    }

    #[test]
    fn test_working_esc_above_border() {
        // Simple "esc to interrupt" without wrapper chars
        let content = "esc to interrupt\n─────\n❯ hello";
        assert_eq!(detect_status(content), ClaudeCodeStatus::Working);
    }

    #[test]
    fn test_idle() {
        // Border directly above prompt
        let content = "● Done\n─────\n❯ hello";
        assert_eq!(detect_status(content), ClaudeCodeStatus::Idle);
    }

    #[test]
    fn test_false_positive_from_scrollback() {
        // Old output contains "interrupt", but current prompt does NOT show interrupt message.
        // Should be Idle, not Working - verifies we only search near prompt
        let content = "Some old output mentioning interrupt\n\n─────\n❯ ready";
        assert_eq!(detect_status(content), ClaudeCodeStatus::Idle);
    }

    #[test]
    fn test_no_border_above_prompt() {
        // Border exists but not directly above prompt - should be unknown
        let content = "─────\nsome text\n❯ hello";
        assert_eq!(detect_status(content), ClaudeCodeStatus::Unknown);
    }

    #[test]
    fn test_waiting_input() {
        let content = "Delete files? [y/n]";
        assert_eq!(detect_status(content), ClaudeCodeStatus::WaitingInput);
    }

    #[test]
    fn test_unknown() {
        let content = "random stuff";
        assert_eq!(detect_status(content), ClaudeCodeStatus::Unknown);
    }

    #[test]
    fn test_interrupt_below_prompt() {
        // Interrupt message appears below the prompt/separator (real-world layout)
        let content = "────────────────────\n❯ \n────────────────────\n  ⏵⏵ bypass permissions on (shift+tab to cycle) · esc to interrupt";
        assert_eq!(detect_status(content), ClaudeCodeStatus::Working);
    }

    #[test]
    fn test_truncated_interrupt() {
        // Truncated message due to narrow window: "esc to…"
        let content = "────────────────────\n❯ \n────────────────────\n  ⏵⏵ bypass permissions on (shift+tab to cycle) · esc to…";
        assert_eq!(detect_status(content), ClaudeCodeStatus::Working);
    }
}
