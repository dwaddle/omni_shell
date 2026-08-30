use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio, Child};
use std::borrow::Cow;
use std::collections::HashMap;
use std::io::{self, Read, Write};
use glob::glob;
use shellexpand;
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

fn get_config_path() -> PathBuf {
    let mut path = PathBuf::from(shellexpand::tilde("~/.config/omni").into_owned());
    fs::create_dir_all(&path).unwrap_or(());
    path.push("omni.toml");
    path
}

fn get_history_path() -> PathBuf {
    let mut path = PathBuf::from(shellexpand::tilde("~/.config/omni").into_owned());
    fs::create_dir_all(&path).unwrap_or(());
    path.push("history.txt");
    path
}

impl ShellState {
    fn new() -> Self {
        let config_path = get_config_path();
        let config = match fs::read_to_string(&config_path) {
            Ok(content) => toml::from_str(&content).unwrap_or_else(|_| OmniConfig::default()),
            Err(_) => OmniConfig::default(),
        };
        ShellState { config, background_jobs: Vec::new() }
    }

    fn clean_jobs(&mut self) {
        self.background_jobs.retain_mut(|(child, _)| {
            match child.try_wait() {
                Ok(Some(_)) => false, 
                Ok(None) => true,           
                Err(_) => false,            
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

    fn expand_args(raw_args: Vec<String>) -> Vec<String> {
        let mut expanded = Vec::new();
        for arg in raw_args {
            let tilde_expanded = shellexpand::tilde(&arg).to_string();
            if tilde_expanded.contains('*') || tilde_expanded.contains('?') {
                if let Ok(paths) = glob(&tilde_expanded) {
                    let mut matched_any = false;
                    for path in paths.flatten() {
                        expanded.push(path.to_string_lossy().into_owned());
                        matched_any = true;
                    }
                    if !matched_any { expanded.push(tilde_expanded); }
                } else {
                    expanded.push(tilde_expanded);
                }
            } else {
                expanded.push(tilde_expanded);
            }
        }
        expanded
    }

    fn execute_command(&mut self, input: &str) {
        self.clean_jobs(); 
        
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
        let original_command_string = cmd_str.clone(); 

        for (i, part) in pipe_parts.iter().enumerate() {
            let raw_args: Vec<String> = part.split_whitespace().map(|s| s.to_string()).collect();
            if raw_args.is_empty() { continue; }
            
            let mut args = raw_args;
            if let Some(alias_val) = self.config.aliases.get(&args[0]) {
                let mut alias_args: Vec<String> = alias_val.split_whitespace().map(|s| s.to_string()).collect();
                alias_args.extend(args.into_iter().skip(1));
                args = alias_args;
            }

            let mut final_args = Self::expand_args(args);
            if final_args.is_empty() { continue; }
            let cmd = final_args.remove(0);

            if cmd == "cd" {
                let new_dir = if final_args.len() > 0 { &final_args[0] } else { "/" };
                let root = std::path::Path::new(new_dir);
                if let Err(e) = env::set_current_dir(&root) { eprintln!("cd fout: {}", e); }
                continue;
            }
            if cmd == "jobs" {
                self.show_jobs();
                continue;
            }
            if cmd == "exit" {
                std::process::exit(0);
            }

            if cmd == "jget" {
                if let Some(key) = final_args.get(0) {
                    if let Some(mut prev) = previous_command.take() {
                        if let Some(mut stdout) = prev.stdout.take() {
                            let mut json_data = String::new();
                            let _ = stdout.read_to_string(&mut json_data);
                            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&json_data) {
                                if let Some(val) = v.get(key) {
                                    println!("{}", val);
                                } else {
                                    eprintln!("Key '{}' niet gevonden in JSON", key);
                                }
                            } else {
                                eprintln!("Input was geen geldige JSON");
                            }
                        }
                    }
                }
                continue;
            }

            let mut command = Command::new(&cmd);
            if !final_args.is_empty() {
                command.args(&final_args);
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

// ---------------- Helper ----------------
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
        Cow::Borrowed(prompt) 
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

// ---------------- Prompt Builder ----------------
fn build_prompt() -> String {
    let current_dir = env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("/"));
    let mut dir_str = current_dir.to_string_lossy().to_string();
    if let Ok(home) = env::var("HOME") {
        if dir_str.starts_with(&home) {
            dir_str = dir_str.replacen(&home, "~", 1);
        }
    }

    let mut git_branch = String::new();
    if let Ok(output) = Command::new("git").args(["rev-parse", "--abbrev-ref", "HEAD"]).stderr(Stdio::null()).output() {
        if output.status.success() {
            let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
            git_branch = format!(" \x1b[1;33mgit:({})\x1b[0m", branch);
        }
    }

    format!("\n\x1b[1;34m{}\x1b[0m{}\n\x1b[1;32momni>\x1b[0m ", dir_str, git_branch)
}

fn main() -> rustyline::Result<()> {
    let config_path = get_config_path();
    if !config_path.exists() {
        let default_toml = "[aliases]\nls = \"eza --icons=always --color=always\"\nll = \"eza -la --icons=always --color=always\"\nupdate = \"sudo pacman -Syu\"\n";
        let _ = fs::write(&config_path, default_toml);
    }

    let mut state = ShellState::new();
    let config = Config::builder().edit_mode(EditMode::Vi).build();
    let h = OmniHelper {
        completer: FilenameCompleter::new(),
        highlighter: MatchingBracketHighlighter::new(),
        hinter: HistoryHinter::new(),
    };
    let mut rl = Editor::<OmniHelper, rustyline::history::DefaultHistory>::with_config(config)?;
    rl.set_helper(Some(h));
    
    let history_path = get_history_path();
    let _ = rl.load_history(&history_path);

    loop {
        let prompt = build_prompt();
        let readline = rl.readline(&prompt);
        match readline {
            Ok(line) => {
                let input = line.trim();
                if input.is_empty() { continue; }
                let _ = rl.add_history_entry(input);
                state.execute_command(input);
            },
            Err(ReadlineError::Interrupted) => { println!("^C"); },
            Err(ReadlineError::Eof) => { break; },
            Err(err) => { println!("Error: {:?}", err); break; }
        }
    }
    
    let _ = rl.save_history(&history_path);
    Ok(())
}
