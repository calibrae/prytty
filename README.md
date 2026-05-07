<p align="center"><img src="assets/logo.png" alt="prytty" width="180"/></p>

# Prytty

Syntax highlighting and pretty-printing for terminal output. 475KB binary, zero heavy dependencies.

Third child of the YTT dynasty ([Hermytt](https://github.com/yttfam/hermytt) | [Crytter](https://github.com/yttfam/crytter) | **Prytty**).

## Install

```sh
cargo install --path prytty-cli
```

## Usage

```sh
# Pipe anything
cat main.rs | prytty
kubectl get pods -o json | prytty --format
git diff | prytty

# Stream mode (real-time, line-by-line)
tail -f /var/log/syslog | prytty --stream
journalctl -f | prytty --stream
cargo build 2>&1 | prytty --stream -l rust

# From file
prytty src/main.rs

# Explicit language
echo '{"key": "value"}' | prytty -l json --format

# Force color mode
prytty --color true main.rs    # truecolor (24-bit)
prytty --color 256 main.rs     # 256-color
prytty --color 16 main.rs      # 16-color
prytty --color none main.rs    # no color
```

## What it highlights

| Language | Detection |
|----------|-----------|
| Rust | `.rs`, shebang, `fn`/`let`/`use` heuristics |
| Python | `.py`, shebang, `def`/`import` heuristics |
| JSON | `.json`, `{"`/`[` content detection |
| YAML | `.yaml`/`.yml`, `---` marker |
| TOML | `.toml`, `[section]` pattern |
| Diff | `.diff`/`.patch`, `diff --git` prefix |
| Logs | `.log`, timestamp patterns (ISO, syslog) |
| Generic | Fallback: strings, numbers, brackets |

Auto-detects from file extension, shebang line, or content heuristics. Use `-l <lang>` to override.

## Features

- **8 regex-based grammars** -- no tree-sitter, no syntect, no regex crate
- **Auto-detection** -- extension, shebang, content analysis
- **Streaming mode** -- `--stream` for real-time pipes (`tail -f`, `kubectl logs -f`)
- **JSON pretty-printing** -- `--format` reformats minified JSON with colors
- **Truecolor / 256 / 16-color** -- auto-detects from `$COLORTERM` and `$TERM`
- **ANSI sanitization** -- strips pre-existing escape sequences to prevent terminal injection
- **475KB binary** -- aggressive release profile (`opt-level=z`, LTO, strip)
- **324 tests** -- unit, integration, E2E, adversarial, OWASP-reviewed

## Security

Prytty processes untrusted input from pipes. By default, it strips all pre-existing ANSI escape sequences from input before highlighting. This prevents:

- Terminal title injection (OSC sequences)
- Clipboard hijacking (OSC 52)
- Arbitrary terminal control via DCS/CSI sequences

Use `--no-sanitize` to pass through original escape sequences if you trust the source.

Input size is capped at 512MB to prevent OOM from unbounded reads.

## Architecture

```
prytty/
├── prytty-core/      # Detection, themes, ANSI output, sanitization
├── prytty-syntax/    # Regex-based grammars (Rust, Python, JSON, ...)
├── prytty-formats/   # Structured data formatting (JSON pretty-print)
└── prytty-cli/       # CLI binary (pipe mode, file mode, streaming)
```

## As a library

```rust
use prytty_core::{detect_language, AnsiWriter};
use prytty_syntax::tokenize;

let lang = detect_language(None, "fn main() {}");
let tokens = tokenize(lang, "fn main() {}");
let writer = AnsiWriter::auto();
let colored = writer.render(&tokens);
print!("{colored}");
```

## Theme

Default theme is VS Code Dark+ inspired, optimized for dark terminal backgrounds. Customizable themes are on the roadmap.

## License

MIT
