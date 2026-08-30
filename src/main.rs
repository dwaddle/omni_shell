use std::env;
use std::process::{Command, Stdio, Child};
use std::borrow::Cow;
use rustyline::error::ReadlineError;
use rustyline::{Config, EditMode, Editor, Context};
use rustyline::hint::{Hinter, HistoryHinter};
use rustyline::highlight::{Highlighter, MatchingBracketHighlighter, CmdKind};
use rustyline::completion::{Completer, FilenameCompleter, Pair};
use rustyline::validate::Validator;
use rustyline::Helper;

#[derive(Helper)]
struct OmniHelper {
    completer: FilenameCompleter,
    highlighter: MatchingBracketHighlighter,
    hinter: HistoryHinter,
}

impl Completer for OmniHelper {
    type Candidate = Pair;
    fn complete(&self, line: &str, pos: usize, ctx: &Context<'_>) -> rustyline::Result<(usize, Vec<Pair>)> {
        self.completer.complete(line, pos, ctx)
    }
}

impl Hinter for OmniHelper {
    type Hint = String;
    fn hint(&self, line: &str, pos: usize, ctx: &Context<'_>) -> Option<String> {
        self.hinter.hint(line, pos, ctx)
    }
}

impl Highlighter for OmniHelper {
    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(&'s self, prompt: &'p str, _default: bool) -> Cow<'b, str> {
        Cow::Owned(format!("\x1b[1;32m{}\x1b[0m", prompt))
    }

    fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
        Cow::Owned(format!("\x1b[90m{}\x1b[0m", hint))
    }

    fn highlight<'l>(&self, line: &'l str, pos: usize) -> Cow<'l, str> {
        self.highlighter.highlight(line, pos)
    }

    fn highlight_char(&self, line: &str, pos: usize, kind: CmdKind) -> bool {
        self.highlighter.highlight_char(line, pos, kind)
    }
}

impl Validator for OmniHelper {}

fn execute_command(input: &str) {
    let mut is_background = false;
    let mut cmd_str = input.trim().to_string();
    
    if cmd_str.ends_with('&') {
        is_background = true;
        cmd_str.pop();
        cmd_str = cmd_str.trim().to_string();
    }

    let pipe_parts: Vec<&str> = cmd_str.split('|').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
    if pipe_parts.is_empty() { return; }

    let mut previous_command: Option<Child> = None;
    let cmd_count = pipe_parts.len();

    for (i, part) in pipe_parts.iter().enumerate() {
        let mut args = part.split_whitespace();
        let cmd = match args.next() {
            Some(c) => c,
            None => continue,
        };

        if cmd == "cd" {
            let new_dir = args.peekable().peek().map_or("/", |x| *x);
            let root = std::path::Path::new(new_dir);
            if let Err(e) = env::set_current_dir(&root) {
                eprintln!("cd fout: {}", e);
            }
            continue;
        }
        if cmd == "exit" {
            std::process::exit(0);
        }

        let mut command = Command::new(cmd);
        command.args(args);

        if let Some(mut prev) = previous_command {
            if let Some(stdout) = prev.stdout.take() {
                command.stdin(Stdio::from(stdout));
            }
        }

        if i < cmd_count - 1 {
            command.stdout(Stdio::piped());
        }

        match command.spawn() {
            Ok(child) => {
                previous_command = Some(child);
            }
            Err(e) => {
                eprintln!("omni: commando niet gevonden of gefaald: {} ({})", cmd, e);
                previous_command = None;
                break;
            }
        }
    }

    if let Some(mut final_child) = previous_command {
        if !is_background {
            let _ = final_child.wait();
        } else {
            println!("[Job draait in de achtergrond met PID {}]", final_child.id());
        }
    }
}

fn main() -> rustyline::Result<()> {
    let config = Config::builder()
        .edit_mode(EditMode::Vi)
        .build();

    let h = OmniHelper {
        completer: FilenameCompleter::new(),
        highlighter: MatchingBracketHighlighter::new(),
        hinter: HistoryHinter::new(),
    };

    let mut rl = Editor::<OmniHelper, rustyline::history::DefaultHistory>::with_config(config)?;
    rl.set_helper(Some(h));

    if rl.load_history("history.txt").is_err() {
        // Geen history gevonden
    }

    loop {
        let readline = rl.readline("omni> ");
        match readline {
            Ok(line) => {
                let input = line.trim();
                if input.is_empty() {
                    continue;
                }
                
                let _ = rl.add_history_entry(input);
                execute_command(input);
            },
            Err(ReadlineError::Interrupted) => {
                println!("^C");
            },
            Err(ReadlineError::Eof) => {
                break;
            },
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
    }
    
    rl.save_history("history.txt")?;
    Ok(())
}
