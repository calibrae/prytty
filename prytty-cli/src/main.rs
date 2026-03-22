use clap::Parser;
use is_terminal::IsTerminal;
use prytty_core::{detect_language, strip_ansi, AnsiWriter, ColorMode, Language};
use prytty_formats::{format_diff_side_by_side, format_json};
use prytty_syntax::tokenize;
use std::io::{self, BufRead, Read, Write};
use std::path::PathBuf;

/// Maximum input size for batch mode: 512 MiB.
const MAX_INPUT_SIZE: u64 = 512 * 1024 * 1024;

/// Lines to buffer for auto-detection in stream mode.
const DETECT_LINES: usize = 5;

#[derive(Parser)]
#[command(name = "prytty", about = "Syntax highlighting for terminal output")]
struct Cli {
    /// File to highlight (reads stdin if omitted)
    file: Option<PathBuf>,

    /// Language hint (auto-detected if omitted)
    #[arg(short, long)]
    language: Option<String>,

    /// Color mode: true, 256, 16, none
    #[arg(long)]
    color: Option<String>,

    /// Format/pretty-print structured data (JSON)
    #[arg(short, long)]
    format: bool,

    /// Side-by-side diff view with word-level highlighting
    #[arg(short = 'S', long)]
    side_by_side: bool,

    /// Terminal width override (auto-detected if omitted)
    #[arg(short = 'W', long)]
    width: Option<u16>,

    /// Stream mode: highlight line-by-line without buffering all input.
    /// Ideal for `tail -f | prytty --stream` or long-running pipes.
    #[arg(short, long)]
    stream: bool,

    /// Theme name
    #[arg(short, long, default_value = "dark+")]
    theme: String,

    /// Allow pre-existing ANSI escape sequences in input to pass through
    /// (by default they are stripped to prevent terminal injection)
    #[arg(long)]
    no_sanitize: bool,
}

fn main() {
    let cli = Cli::parse();

    let color_mode = detect_color_mode(cli.color.as_deref());

    // Stream mode: line-by-line from stdin
    if cli.stream {
        if cli.file.is_some() {
            eprintln!("prytty: --stream only works with stdin");
            std::process::exit(1);
        }
        run_stream(&cli, color_mode);
        return;
    }

    // Batch mode: read all, detect, process, output
    run_batch(&cli, color_mode);
}

fn detect_color_mode(color: Option<&str>) -> ColorMode {
    match color {
        Some("true") | Some("truecolor") | Some("rgb") => ColorMode::TrueColor,
        Some("256") => ColorMode::Color256,
        Some("16") => ColorMode::Color16,
        Some("none") | Some("off") => ColorMode::None,
        _ => {
            if io::stdout().is_terminal() {
                ColorMode::detect()
            } else {
                ColorMode::None
            }
        }
    }
}

fn run_batch(cli: &Cli, color_mode: ColorMode) {
    let input = match &cli.file {
        Some(path) => {
            match std::fs::metadata(path) {
                Ok(meta) if meta.len() > MAX_INPUT_SIZE => {
                    eprintln!(
                        "prytty: {}: file too large ({} bytes, max {})",
                        path.display(),
                        meta.len(),
                        MAX_INPUT_SIZE
                    );
                    std::process::exit(1);
                }
                Err(e) => {
                    eprintln!("prytty: {}: {}", path.display(), e);
                    std::process::exit(1);
                }
                _ => {}
            }
            std::fs::read_to_string(path).unwrap_or_else(|e| {
                eprintln!("prytty: {}: {}", path.display(), e);
                std::process::exit(1);
            })
        }
        None => {
            let mut buf = String::new();
            io::stdin()
                .take(MAX_INPUT_SIZE)
                .read_to_string(&mut buf)
                .unwrap_or_else(|e| {
                    eprintln!("prytty: stdin: {}", e);
                    std::process::exit(1);
                });
            buf
        }
    };

    let input = if cli.no_sanitize { input } else { strip_ansi(&input) };

    let lang = cli
        .language
        .as_deref()
        .and_then(Language::from_name)
        .unwrap_or_else(|| detect_language(cli.file.as_deref(), &input));

    // Side-by-side diff mode
    if cli.side_by_side && lang == Language::Diff {
        let term_width = cli.width.unwrap_or_else(|| {
            terminal_size::terminal_size()
                .map(|(w, _)| w.0)
                .unwrap_or(120)
        });
        let output = format_diff_side_by_side(&input, term_width, color_mode);
        let stdout = io::stdout();
        let mut out = stdout.lock();
        let _ = out.write_all(output.as_bytes());
        return;
    }

    let input = if cli.format && lang == Language::Json {
        format_json(&input)
    } else {
        input
    };

    let tokens = tokenize(lang, &input);
    let writer = AnsiWriter::new(color_mode, Default::default());
    let output = writer.render(&tokens);

    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = out.write_all(output.as_bytes());
}

fn run_stream(cli: &Cli, color_mode: ColorMode) {
    let stdin = io::stdin();
    let reader = stdin.lock();
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let writer = AnsiWriter::new(color_mode, Default::default());
    let sanitize = !cli.no_sanitize;

    // If language is specified, skip detection and stream immediately
    if let Some(lang) = cli.language.as_deref().and_then(Language::from_name) {
        for line in reader.lines() {
            let line = match line {
                Ok(l) => l,
                Err(_) => break,
            };
            highlight_line(&writer, &mut out, &line, lang, sanitize);
        }
        return;
    }

    // No language specified: buffer first N lines for detection, then stream
    let mut lines_iter = reader.lines();
    let mut buffer = Vec::with_capacity(DETECT_LINES);

    for _ in 0..DETECT_LINES {
        match lines_iter.next() {
            Some(Ok(line)) => buffer.push(line),
            Some(Err(_)) => break,
            None => break,
        }
    }

    // Detect from buffered content
    let preview = buffer.join("\n");
    let lang = detect_language(None, &preview);

    // Output buffered lines
    for line in &buffer {
        highlight_line(&writer, &mut out, line, lang, sanitize);
    }
    drop(buffer);

    // Stream the rest
    for line in lines_iter {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        highlight_line(&writer, &mut out, &line, lang, sanitize);
    }
}

fn highlight_line(
    writer: &AnsiWriter,
    out: &mut impl Write,
    line: &str,
    lang: Language,
    sanitize: bool,
) {
    let line = if sanitize {
        strip_ansi(line)
    } else {
        line.to_string()
    };
    let tokens = tokenize(lang, &line);
    let rendered = writer.render(&tokens);
    let _ = out.write_all(rendered.as_bytes());
    let _ = out.write_all(b"\n");
    let _ = out.flush();
}
