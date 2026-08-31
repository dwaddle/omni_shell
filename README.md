# Omni Shell

A fast, modern, and highly capable Unix shell written in Rust.

`omni_shell` is built on top of [reedline](https://github.com/nushell/reedline) (the engine behind Nushell) to provide an advanced, interactive command-line experience with modern defaults.

## Features

- **Advanced Visual Menus**: 
  - `[TAB]` triggers a Columnar Menu for real-time filesystem path completion.
  - `[Ctrl-R]` triggers a visual List Menu for reverse-searching command history.
- **Vim Mode by Default**: First-class support for `evil-mode` equivalent Vi keybindings. Includes a live prompt indicator (`[N]` for Normal, `omni>` for Insert).
- **Custom AST Pipeline Execution**: Robust parser that correctly handles shell pipelines separated by `&&`, `||`, and `;` with full short-circuiting logic.
- **Native Zoxide Integration**: Built-in `z` command directly interfaces with your [Zoxide](https://github.com/ajeetdsouza/zoxide) database for lightning-fast directory jumping without needing external shell hooks.
- **Smart Quoting & Expansion**: Bulletproof quoting and variable expansion utilizing `shlex` and `shellexpand`. 
- **Built-in Aliases & Export**: First-class support for environment variable exporting and command aliasing via `omni.toml`.

## Installation

Ensure you have Rust and Cargo installed, then run:

```bash
cargo install --path .
```

This will compile `omni_shell` and place it in your `~/.cargo/bin/` directory.

## Configuration

`omni_shell` reads its configuration from an `omni.toml` file (or default locations).
It supports:
- Defining aliases (e.g., `alias cd="z"` natively translates to the zoxide built-in).
- Environment variables.

## Technical Details

- **Backend**: Reedline (replaces older rustyline backend for superior UI rendering).
- **AST Parser**: Custom written tokenizer and executor (`src/parser.rs` and `src/executor.rs`) to evaluate and execute logic chains.
- **Concurrency**: Background jobs support (`&`).

## License
MIT
