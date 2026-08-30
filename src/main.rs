use std::env;
use std::fs;
use std::process::{Command, Stdio, Child};
use std::borrow::Cow;
use std::collections::HashMap;
use serde::Deserialize;

use rustyline::error::ReadlineError;
use rustyline::{Config, EditMode, Editor, Context};
use rustyline::hint::{Hinter, HistoryHinter};
use rustyline::highlight::{Highlighter, MatchingBracketHighlighter, CmdKind};
use rustyline::completion::{Completer, FilenameCompleter, Pair};
use rustyline::validate::Validator;
use rustyline::Helper;

#[derive(Deserialize, Default, Debug)]
struct OmniConfig {
    #[serde(default)]
    aliases: HashMap<String, String>,
}

struct ShellState {
    config: OmniConfig,
    background_jobs: Vec<(Child, String)>,
}

impl ShellState {
    fn new() -> Self {
        let config = match fs::read_to_string("omni.toml") {
            Ok(content) => toml::from_str(&content).unwrap_or_else(|err| {
                eprintln!("Fout bij parsen omni.toml: {}", err);
                OmniConfig::default()
            }),
            Err(_) => OmniConfig::default(),
        };

        ShellState {
            config,
            background_jobs: Vec::new(),
        }
    }

    fn clean_jobs(&mut self) {
        // Verwijder jobs die inmiddels klaar zijn
        self.background_jobs.retain_mut(|(child, _)| {
            match child.try_wait() {
                Ok(Some(_status)) => false, // Klaar, dus verwijder uit lijst
                Ok(None) => true,           // Nog bezig, bewaar
                Err(_) => false,            // Error, gooi weg
            }
        });
    }

    fn show_jobs(&mut self) {
        self.clean_jobs();
        if self.background_jobs.is_empty() {
            println!("Geen achtergrondtaken actief.");
        } else {
            for (child, cmd) in &self.background_jobs {
                println!("[{}] Draaiend: {}", child.id(), cmd);
            }
        }
    }

    fn execute_command(&mut self, input: &str) {
        self.clean_jobs(); // Ruim afgeronde achtergrondtaken op
        
        let mut is_background = false;
        let mut cmd_str = input.trim().to_string();
        
        if cmd_str.ends_with('&') {
            is_background = true;
            cmd_str.pop();
            cmd_str = cmd_str.trim().to_string();
        }

        // Resolveer aliases voor het eerste commando in de pipeline
        // Opmerking: In een volledige iteratie zouden we aliases per pipe-part oplossen.
        let pipe_parts: Vec<&str> = cmd_str.split('|').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
        if pipe_parts.is_empty() { return; }

        let mut previous_command: Option<Child> = None;
        let cmd_count = pipe_parts.len();
        
        let original_command_string = cmd_str.clone(); // Voor job lijst

        for (i, part) in pipe_parts.iter().enumerate() {
            let mut args: Vec<String> = part.split_whitespace().map(|s| s.to_string()).collect();
            if args.is_empty() { continue; }
            
            // Check alias alleen voor het commando zelf
            if let Some(alias_val) = self.config.aliases.get(&args[0]) {
                let mut alias_args: Vec<String> = alias_val.split_whitespace().map(|s| s.to_string()).collect();
                alias_args.extend(args.into_iter().skip(1));
                args = alias_args;
            }

            let cmd = args[0].clone();
            
            if cmd == "cd" {
                let new_dir = if args.len() > 1 { &args[1] } else { "/" };
                let root = std::path::Path::new(new_dir);
                if let Err(e) = env::set_current_dir(&root) {
                    eprintln!("cd fout: {}", e);
                }
                continue;
            }
            if cmd == "jobs" {
                self.show_jobs();
                continue;
            }
            if cmd == "exit" {
                std::process::exit(0);
            }

            let mut command = Command::new(&cmd);
            if args.len() > 1 {
                command.args(&args[1..]);
            }

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
                println!("[Job in de achtergrond gestart met PID {}]", final_child.id());
                self.background_jobs.push((final_child, original_command_string));
            }
        }
    }
}

// ---------------- Rustyline Configuratie ----------------

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

fn main() -> rustyline::Result<()> {
    // Schrijf een default configuratie als die niet bestaat
    if !std::path::Path::new("omni.toml").exists() {
        let default_toml = "[aliases]\nll = \"ls -la\"\nupdate = \"sudo pacman -Syu\"\n";
        fs::write("omni.toml", default_toml).unwrap();
    }

    let mut state = ShellState::new();

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
                state.execute_command(input);
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
