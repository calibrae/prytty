use std::fmt::Write;

use prytty_core::ColorMode;

/// Side-by-side diff renderer.
/// Takes unified diff input, outputs a two-column view with:
/// - Line numbers on each side
/// - Word-level diff highlighting within changed lines
/// - Red for removals (left), green for additions (right)
pub fn format_diff_side_by_side(input: &str, term_width: u16, color_mode: ColorMode) -> String {
    let hunks = parse_unified_diff(input);
    let c = Colors::new(color_mode);

    // Line number gutter width: max digits needed
    let max_line = hunks.iter().flat_map(|h| {
        h.pairs.iter().flat_map(|p| {
            let mut nums = Vec::new();
            if let Some(n) = p.old_line { nums.push(n); }
            if let Some(n) = p.new_line { nums.push(n); }
            nums
        })
    }).max().unwrap_or(1);
    let gutter_w = digit_count(max_line);

    // Layout: [gutter] [space] [left content] [separator] [gutter] [space] [right content]
    // separator = " │ " (3 chars)
    let sep_w = 3;
    let overhead = (gutter_w + 1) * 2 + sep_w; // two gutters + two spaces + separator
    let content_w = if term_width as usize > overhead + 10 {
        (term_width as usize - overhead) / 2
    } else {
        40
    };

    let mut out = String::new();

    for hunk in &hunks {
        // File headers
        if let Some(ref header) = hunk.file_header {
            let _ = writeln!(out, "{}{}{}", c.header, header, c.reset);
        }
        // Hunk header
        let _ = writeln!(out, "{}{}{}", c.hunk, hunk.header, c.reset);

        for pair in &hunk.pairs {
            match pair.kind {
                PairKind::Context => {
                    let text = pair.old_text.as_deref().unwrap_or("");
                    let left = truncate(text, content_w);
                    let right = truncate(text, content_w);
                    let _ = writeln!(
                        out,
                        "{}{:>gw$}{} {:<cw$} {} {}{:>gw$}{} {}",
                        c.line_no, pair.old_line.unwrap_or(0), c.reset,
                        left,
                        c.sep,
                        c.line_no, pair.new_line.unwrap_or(0), c.reset,
                        right,
                        gw = gutter_w, cw = content_w,
                    );
                }
                PairKind::Changed => {
                    let old = pair.old_text.as_deref().unwrap_or("");
                    let new = pair.new_text.as_deref().unwrap_or("");
                    let (left_parts, right_parts) = word_diff(old, new);
                    let left_rendered = render_word_diff(&left_parts, &c.del, &c.del_word, &c.reset, content_w);
                    let right_rendered = render_word_diff(&right_parts, &c.add, &c.add_word, &c.reset, content_w);
                    let left_num = pair.old_line.map(|n| format!("{:>gw$}", n, gw = gutter_w)).unwrap_or(" ".repeat(gutter_w));
                    let right_num = pair.new_line.map(|n| format!("{:>gw$}", n, gw = gutter_w)).unwrap_or(" ".repeat(gutter_w));
                    let _ = writeln!(
                        out,
                        "{}{}{} {} {} {}{}{} {}",
                        c.line_no, left_num, c.reset,
                        left_rendered,
                        c.sep,
                        c.line_no, right_num, c.reset,
                        right_rendered,
                    );
                }
                PairKind::Delete => {
                    let old = pair.old_text.as_deref().unwrap_or("");
                    let left = format!("{}{}{}", c.del, truncate(old, content_w), c.reset);
                    let left_num = pair.old_line.map(|n| format!("{:>gw$}", n, gw = gutter_w)).unwrap_or(" ".repeat(gutter_w));
                    let _ = writeln!(
                        out,
                        "{}{}{} {:<cw$} {} {}{}{}",
                        c.line_no, left_num, c.reset,
                        left,
                        c.sep,
                        c.dim, " ".repeat(gutter_w), c.reset,
                        cw = content_w + c.del.len() + c.reset.len(),
                    );
                }
                PairKind::Add => {
                    let new = pair.new_text.as_deref().unwrap_or("");
                    let right = format!("{}{}{}", c.add, truncate(new, content_w), c.reset);
                    let right_num = pair.new_line.map(|n| format!("{:>gw$}", n, gw = gutter_w)).unwrap_or(" ".repeat(gutter_w));
                    let _ = writeln!(
                        out,
                        "{}{}{} {:<cw$} {} {}{}{} {}",
                        c.dim, " ".repeat(gutter_w), c.reset,
                        "",
                        c.sep,
                        c.line_no, right_num, c.reset,
                        right,
                        cw = content_w,
                    );
                }
            }
        }
    }

    out
}

// --- Diff parsing ---

#[derive(Debug)]
struct Hunk {
    file_header: Option<String>,
    header: String,
    pairs: Vec<LinePair>,
}

#[derive(Debug)]
struct LinePair {
    kind: PairKind,
    old_line: Option<usize>,
    new_line: Option<usize>,
    old_text: Option<String>,
    new_text: Option<String>,
}

#[derive(Debug, PartialEq)]
enum PairKind {
    Context,
    Changed, // old + new (side by side with word diff)
    Delete,  // old only
    Add,     // new only
}

fn parse_unified_diff(input: &str) -> Vec<Hunk> {
    let lines: Vec<&str> = input.lines().collect();
    let mut hunks = Vec::new();
    let mut i = 0;
    let mut file_header: Option<String> = None;

    while i < lines.len() {
        let line = lines[i];

        // File headers
        if line.starts_with("diff ") || line.starts_with("index ") {
            i += 1;
            continue;
        }
        if line.starts_with("--- ") {
            let mut header = line.to_string();
            if i + 1 < lines.len() && lines[i + 1].starts_with("+++ ") {
                header = format!("{}\n{}", line, lines[i + 1]);
                i += 1;
            }
            file_header = Some(header);
            i += 1;
            continue;
        }

        // Hunk header: @@ -old_start,old_count +new_start,new_count @@
        if line.starts_with("@@ ") {
            let (old_start, new_start) = parse_hunk_header(line);
            let mut pairs = Vec::new();
            let mut old_line = old_start;
            let mut new_line = new_start;
            i += 1;

            // Collect removals and additions in runs, then pair them
            while i < lines.len() && !lines[i].starts_with("@@ ") && !lines[i].starts_with("diff ") {
                let l = lines[i];

                if l.starts_with(' ') || l.is_empty() {
                    // Flush any pending run first, then add context
                    let text = if l.is_empty() { "" } else { &l[1..] };
                    pairs.push(LinePair {
                        kind: PairKind::Context,
                        old_line: Some(old_line),
                        new_line: Some(new_line),
                        old_text: Some(text.to_string()),
                        new_text: None,
                    });
                    old_line += 1;
                    new_line += 1;
                    i += 1;
                    continue;
                }

                // Collect a run of -/+ lines
                let mut dels: Vec<(usize, String)> = Vec::new();
                let mut adds: Vec<(usize, String)> = Vec::new();

                while i < lines.len() && lines[i].starts_with('-') && !lines[i].starts_with("--- ") {
                    dels.push((old_line, lines[i][1..].to_string()));
                    old_line += 1;
                    i += 1;
                }
                while i < lines.len() && lines[i].starts_with('+') && !lines[i].starts_with("+++ ") {
                    adds.push((new_line, lines[i][1..].to_string()));
                    new_line += 1;
                    i += 1;
                }

                // Pair them up
                let max_len = dels.len().max(adds.len());
                for j in 0..max_len {
                    let del = dels.get(j);
                    let add = adds.get(j);
                    match (del, add) {
                        (Some((ol, ot)), Some((nl, nt))) => {
                            pairs.push(LinePair {
                                kind: PairKind::Changed,
                                old_line: Some(*ol),
                                new_line: Some(*nl),
                                old_text: Some(ot.clone()),
                                new_text: Some(nt.clone()),
                            });
                        }
                        (Some((ol, ot)), None) => {
                            pairs.push(LinePair {
                                kind: PairKind::Delete,
                                old_line: Some(*ol),
                                new_line: None,
                                old_text: Some(ot.clone()),
                                new_text: None,
                            });
                        }
                        (None, Some((nl, nt))) => {
                            pairs.push(LinePair {
                                kind: PairKind::Add,
                                old_line: None,
                                new_line: Some(*nl),
                                old_text: None,
                                new_text: Some(nt.clone()),
                            });
                        }
                        (None, None) => {}
                    }
                }
                continue;
            }

            hunks.push(Hunk {
                file_header: file_header.take(),
                header: line.to_string(),
                pairs,
            });
            continue;
        }

        i += 1;
    }

    hunks
}

fn parse_hunk_header(line: &str) -> (usize, usize) {
    // @@ -old_start[,old_count] +new_start[,new_count] @@
    let mut old_start = 1;
    let mut new_start = 1;

    if let Some(rest) = line.strip_prefix("@@ -") {
        if let Some(plus_pos) = rest.find(" +") {
            let old_part = &rest[..plus_pos];
            old_start = old_part.split(',').next()
                .and_then(|s| s.parse().ok())
                .unwrap_or(1);

            let after_plus = &rest[plus_pos + 2..];
            let new_part = if let Some(sp) = after_plus.find(' ') {
                &after_plus[..sp]
            } else {
                after_plus.trim()
            };
            new_start = new_part.split(',').next()
                .and_then(|s| s.parse().ok())
                .unwrap_or(1);
        }
    }

    (old_start, new_start)
}

// --- Word-level diff ---

#[derive(Debug, Clone, PartialEq)]
enum WordChunk<'a> {
    Equal(&'a str),
    Changed(&'a str),
}

fn word_diff<'a>(old: &'a str, new: &'a str) -> (Vec<WordChunk<'a>>, Vec<WordChunk<'a>>) {
    let old_words = split_words(old);
    let new_words = split_words(new);

    // Myers-like LCS on words
    let lcs = lcs_words(&old_words, &new_words);

    let mut old_chunks = Vec::new();
    let mut new_chunks = Vec::new();
    let mut oi = 0;
    let mut ni = 0;

    for (lo, ln) in &lcs {
        // Changed words before this match
        if oi < *lo || ni < *ln {
            for &w in &old_words[oi..*lo] {
                old_chunks.push(WordChunk::Changed(w));
            }
            for &w in &new_words[ni..*ln] {
                new_chunks.push(WordChunk::Changed(w));
            }
        }
        // Matched word
        old_chunks.push(WordChunk::Equal(old_words[*lo]));
        new_chunks.push(WordChunk::Equal(new_words[*ln]));
        oi = lo + 1;
        ni = ln + 1;
    }

    // Remaining
    for &w in &old_words[oi..] {
        old_chunks.push(WordChunk::Changed(w));
    }
    for &w in &new_words[ni..] {
        new_chunks.push(WordChunk::Changed(w));
    }

    (old_chunks, new_chunks)
}

fn split_words(s: &str) -> Vec<&str> {
    let mut words = Vec::new();
    let mut i = 0;
    let bytes = s.as_bytes();

    while i < bytes.len() {
        if bytes[i].is_ascii_whitespace() {
            let start = i;
            while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            words.push(&s[start..i]);
        } else {
            let start = i;
            while i < bytes.len() && !bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            words.push(&s[start..i]);
        }
    }

    words
}

/// Simple LCS on word slices. Returns pairs of (old_idx, new_idx).
fn lcs_words<'a>(old: &[&'a str], new: &[&'a str]) -> Vec<(usize, usize)> {
    let m = old.len();
    let n = new.len();

    if m == 0 || n == 0 {
        return Vec::new();
    }

    // Cap to prevent quadratic blowup on huge diffs
    if m * n > 100_000 {
        return Vec::new();
    }

    // DP table
    let mut dp = vec![vec![0u32; n + 1]; m + 1];
    for i in (0..m).rev() {
        for j in (0..n).rev() {
            if old[i] == new[j] {
                dp[i][j] = dp[i + 1][j + 1] + 1;
            } else {
                dp[i][j] = dp[i + 1][j].max(dp[i][j + 1]);
            }
        }
    }

    // Backtrack
    let mut result = Vec::new();
    let mut i = 0;
    let mut j = 0;
    while i < m && j < n {
        if old[i] == new[j] {
            result.push((i, j));
            i += 1;
            j += 1;
        } else if dp[i + 1][j] >= dp[i][j + 1] {
            i += 1;
        } else {
            j += 1;
        }
    }

    result
}

fn render_word_diff(chunks: &[WordChunk<'_>], base_color: &str, highlight_color: &str, reset: &str, max_width: usize) -> String {
    let mut out = String::new();
    let mut visible_len = 0;

    for chunk in chunks {
        let text = match chunk {
            WordChunk::Equal(t) => t,
            WordChunk::Changed(t) => t,
        };

        let remaining = max_width.saturating_sub(visible_len);
        if remaining == 0 {
            break;
        }

        let display_text = if text.len() > remaining {
            &text[..remaining]
        } else {
            text
        };

        match chunk {
            WordChunk::Equal(_) => {
                let _ = write!(out, "{}{}{}", base_color, display_text, reset);
            }
            WordChunk::Changed(_) => {
                let _ = write!(out, "{}{}{}", highlight_color, display_text, reset);
            }
        }
        visible_len += display_text.len();
    }

    // Pad to width
    if visible_len < max_width {
        let _ = write!(out, "{}", " ".repeat(max_width - visible_len));
    }

    out
}

// --- Colors ---

struct Colors {
    del: String,
    del_word: String,
    add: String,
    add_word: String,
    header: String,
    hunk: String,
    line_no: String,
    dim: String,
    sep: String,
    reset: String,
}

impl Colors {
    fn new(mode: ColorMode) -> Self {
        match mode {
            ColorMode::None => Self {
                del: String::new(),
                del_word: String::new(),
                add: String::new(),
                add_word: String::new(),
                header: String::new(),
                hunk: String::new(),
                line_no: String::new(),
                dim: String::new(),
                sep: "│".into(),
                reset: String::new(),
            },
            ColorMode::TrueColor => Self {
                del: "\x1b[38;2;250;80;80m".into(),           // red
                del_word: "\x1b[48;2;80;0;0m\x1b[38;2;255;150;150m".into(), // bright red on dark red bg
                add: "\x1b[38;2;80;250;120m".into(),           // green
                add_word: "\x1b[48;2;0;60;0m\x1b[38;2;150;255;150m".into(), // bright green on dark green bg
                header: "\x1b[1m\x1b[38;2;220;220;170m".into(), // bold yellow
                hunk: "\x1b[38;2;100;150;224m".into(),          // blue
                line_no: "\x1b[38;2;100;100;100m".into(),       // dim gray
                dim: "\x1b[38;2;60;60;60m".into(),              // very dim
                sep: "\x1b[38;2;60;60;60m│\x1b[0m".into(),     // dim separator
                reset: "\x1b[0m".into(),
            },
            ColorMode::Color256 => Self {
                del: "\x1b[38;5;196m".into(),
                del_word: "\x1b[48;5;52m\x1b[38;5;217m".into(),
                add: "\x1b[38;5;82m".into(),
                add_word: "\x1b[48;5;22m\x1b[38;5;156m".into(),
                header: "\x1b[1m\x1b[38;5;186m".into(),
                hunk: "\x1b[38;5;68m".into(),
                line_no: "\x1b[38;5;240m".into(),
                dim: "\x1b[38;5;236m".into(),
                sep: "\x1b[38;5;236m│\x1b[0m".into(),
                reset: "\x1b[0m".into(),
            },
            ColorMode::Color16 => Self {
                del: "\x1b[31m".into(),
                del_word: "\x1b[1m\x1b[31m".into(),
                add: "\x1b[32m".into(),
                add_word: "\x1b[1m\x1b[32m".into(),
                header: "\x1b[1m\x1b[33m".into(),
                hunk: "\x1b[36m".into(),
                line_no: "\x1b[90m".into(),
                dim: "\x1b[90m".into(),
                sep: "\x1b[90m│\x1b[0m".into(),
                reset: "\x1b[0m".into(),
            },
        }
    }
}

// --- Helpers ---

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        format!("{:<width$}", s, width = max)
    } else {
        format!("{}…", &s[..max.saturating_sub(1)])
    }
}

fn digit_count(n: usize) -> usize {
    if n == 0 { return 1; }
    ((n as f64).log10().floor() as usize) + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_hunk_header_basic() {
        // Returns (old_start, new_start) — the starting line numbers
        assert_eq!(parse_hunk_header("@@ -1,5 +1,7 @@"), (1, 1));
        assert_eq!(parse_hunk_header("@@ -10,3 +12,5 @@ fn main()"), (10, 12));
    }

    #[test]
    fn parse_hunk_header_no_count() {
        assert_eq!(parse_hunk_header("@@ -1 +1 @@"), (1, 1));
    }

    #[test]
    fn split_words_basic() {
        let words = split_words("hello world");
        assert_eq!(words, vec!["hello", " ", "world"]);
    }

    #[test]
    fn split_words_preserves_whitespace() {
        let words = split_words("  foo  bar");
        assert_eq!(words, vec!["  ", "foo", "  ", "bar"]);
    }

    #[test]
    fn word_diff_identical() {
        let (old, new) = word_diff("hello world", "hello world");
        assert!(old.iter().all(|c| matches!(c, WordChunk::Equal(_))));
        assert!(new.iter().all(|c| matches!(c, WordChunk::Equal(_))));
    }

    #[test]
    fn word_diff_one_word_changed() {
        let (old, new) = word_diff("hello world", "hello earth");
        // "hello" and " " should be Equal, "world"/"earth" should be Changed
        assert_eq!(old.last(), Some(&WordChunk::Changed("world")));
        assert_eq!(new.last(), Some(&WordChunk::Changed("earth")));
    }

    #[test]
    fn word_diff_addition() {
        let (old, new) = word_diff("a b", "a b c");
        // old should have Equal("a"), Equal(" "), Equal("b")
        // new should have same plus Changed(" "), Changed("c")
        assert!(new.iter().any(|c| matches!(c, WordChunk::Changed("c"))));
    }

    #[test]
    fn parse_unified_diff_basic() {
        let diff = "\
diff --git a/foo.rs b/foo.rs
index abc..def 100644
--- a/foo.rs
+++ b/foo.rs
@@ -1,3 +1,3 @@
 unchanged
-old line
+new line
";
        let hunks = parse_unified_diff(diff);
        assert_eq!(hunks.len(), 1);
        assert_eq!(hunks[0].pairs.len(), 2); // context + changed
        assert_eq!(hunks[0].pairs[0].kind, PairKind::Context);
        assert_eq!(hunks[0].pairs[1].kind, PairKind::Changed);
    }

    #[test]
    fn parse_unified_diff_delete_only() {
        let diff = "\
--- a/foo
+++ b/foo
@@ -1,3 +1,2 @@
 keep
-removed
 keep
";
        let hunks = parse_unified_diff(diff);
        assert_eq!(hunks[0].pairs.len(), 3);
        assert_eq!(hunks[0].pairs[1].kind, PairKind::Delete);
    }

    #[test]
    fn parse_unified_diff_add_only() {
        let diff = "\
--- a/foo
+++ b/foo
@@ -1,2 +1,3 @@
 keep
+added
 keep
";
        let hunks = parse_unified_diff(diff);
        assert_eq!(hunks[0].pairs.len(), 3);
        assert_eq!(hunks[0].pairs[1].kind, PairKind::Add);
    }

    #[test]
    fn digit_count_works() {
        assert_eq!(digit_count(0), 1);
        assert_eq!(digit_count(1), 1);
        assert_eq!(digit_count(9), 1);
        assert_eq!(digit_count(10), 2);
        assert_eq!(digit_count(99), 2);
        assert_eq!(digit_count(100), 3);
        assert_eq!(digit_count(999), 3);
    }

    #[test]
    fn truncate_short() {
        let t = truncate("hi", 10);
        assert_eq!(t.len(), 10);
        assert!(t.starts_with("hi"));
    }

    #[test]
    fn truncate_long() {
        let t = truncate("hello world this is long", 10);
        assert!(t.ends_with('…'));
        // 9 ASCII chars + '…' (3 bytes) = 12 bytes, but 10 visible chars
        assert_eq!(t.chars().count(), 10);
    }

    #[test]
    fn lcs_empty() {
        let result = lcs_words(&[], &["a"]);
        assert!(result.is_empty());
    }

    #[test]
    fn lcs_identical() {
        let result = lcs_words(&["a", "b", "c"], &["a", "b", "c"]);
        assert_eq!(result, vec![(0, 0), (1, 1), (2, 2)]);
    }

    #[test]
    fn format_no_color() {
        let diff = "\
--- a/foo
+++ b/foo
@@ -1,3 +1,3 @@
 same
-old
+new
";
        let result = format_diff_side_by_side(diff, 80, ColorMode::None);
        assert!(result.contains("same"));
        assert!(result.contains("old"));
        assert!(result.contains("new"));
        assert!(result.contains("│"));
    }

    #[test]
    fn format_truecolor() {
        let diff = "\
--- a/foo
+++ b/foo
@@ -1,2 +1,2 @@
-hello world
+hello earth
";
        let result = format_diff_side_by_side(diff, 100, ColorMode::TrueColor);
        // Should contain ANSI escape sequences
        assert!(result.contains("\x1b["));
        // Should contain the word "earth" somewhere
        assert!(result.contains("earth"));
    }
}
