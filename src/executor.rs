use std::env;
use std::fs::OpenOptions;
use std::process::{Command, Stdio, Child};
use std::io::Read;
use glob::glob;
use shellexpand;
use shlex;
use crate::config::OmniConfig;

pub struct ShellState {
    pub config: OmniConfig,
    pub background_jobs: Vec<(Child, String)>,
}

impl ShellState {
    pub fn new(config: OmniConfig) -> Self {
        ShellState { config, background_jobs: Vec::new() }
    }

    pub fn clean_jobs(&mut self) {
        self.background_jobs.retain_mut(|(child, _)| {
            match child.try_wait() {
                Ok(Some(_)) => false, 
                Ok(None) => true,           
                Err(_) => false,            
            }
        });
    }

    pub fn show_jobs(&mut self) {
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
            // Full expandert ZOWEL tilde (~) als env vars ($VAR)
            let full_expanded = match shellexpand::full(&arg) {
                Ok(cow) => cow.into_owned(),
                Err(_) => arg.clone(),
            };

            if full_expanded.contains('*') || full_expanded.contains('?') {
                if let Ok(paths) = glob(&full_expanded) {
                    let mut matched_any = false;
                    for path in paths.flatten() {
                        expanded.push(path.to_string_lossy().into_owned());
                        matched_any = true;
                    }
                    if !matched_any { expanded.push(full_expanded); }
                } else {
                    expanded.push(full_expanded);
                }
            } else {
                expanded.push(full_expanded);
            }
        }
        expanded
    }

    pub fn execute_command(&mut self, input: &str) {
        self.clean_jobs(); 
        
        let mut is_background = false;
        let mut cmd_str = input.trim().to_string();
        
        if cmd_str.ends_with('&') {
            is_background = true;
            cmd_str.pop();
            cmd_str = cmd_str.trim().to_string();
        }

        // Voor nu nog steeds eenvoudige pipe-splitting
        let pipe_parts: Vec<&str> = cmd_str.split('|').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
        if pipe_parts.is_empty() { return; }

        let mut previous_command: Option<Child> = None;
        let cmd_count = pipe_parts.len();
        let original_command_string = cmd_str.clone(); 

        for (i, part) in pipe_parts.iter().enumerate() {
            // VERVANGING: shlex voor perfecte quote handling
            let raw_args = match shlex::split(part) {
                Some(args) => args,
                None => {
                    eprintln!("omni: syntax fout, ontbrekende quote");
                    return;
                }
            };
            
            if raw_args.is_empty() { continue; }
            
            let mut args = raw_args;
            if let Some(alias_val) = self.config.aliases.get(&args[0]) {
                if let Some(mut alias_args) = shlex::split(alias_val) {
                    alias_args.extend(args.into_iter().skip(1));
                    args = alias_args;
                }
            }

            let mut final_args = Self::expand_args(args);
            if final_args.is_empty() { continue; }
            
            // Redirection extractie
            let mut redirect_out = None;
            let mut append_out = false;
            let mut clean_args = Vec::new();
            
            let mut arg_iter = final_args.into_iter();
            while let Some(arg) = arg_iter.next() {
                if arg == ">" {
                    redirect_out = arg_iter.next();
                    append_out = false;
                } else if arg == ">>" {
                    redirect_out = arg_iter.next();
                    append_out = true;
                } else {
                    clean_args.push(arg);
                }
            }
            
            if clean_args.is_empty() { continue; }
            let cmd = clean_args.remove(0);

            // BUILT-INS
            if cmd == "cd" {
                let new_dir = if clean_args.len() > 0 { &clean_args[0] } else { "/" };
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
            if cmd == "export" {
                if clean_args.is_empty() {
                    for (k, v) in env::vars() { println!("{}={}", k, v); }
                } else {
                    let full = clean_args.join(" ");
                    let parts: Vec<&str> = full.splitn(2, '=').collect();
                    if parts.len() == 2 {
                        unsafe { env::set_var(parts[0], parts[1]); }
                    } else {
                        eprintln!("Gebruik: export VAR=waarde");
                    }
                }
                continue;
            }
            if cmd == "alias" {
                if clean_args.is_empty() {
                    for (k, v) in &self.config.aliases { println!("alias {}='{}'", k, v); }
                } else {
                    let full_arg = clean_args.join(" ");
                    let parts: Vec<&str> = full_arg.splitn(2, '=').collect();
                    if parts.len() == 2 {
                        let key = parts[0].trim().to_string();
                        let value = parts[1].trim().trim_matches('\'').trim_matches('"').to_string();
                        self.config.aliases.insert(key, value);
                    } else {
                        eprintln!("Gebruik: alias naam='commando'");
                    }
                }
                continue;
            }

            if cmd == "jget" {
                if let Some(key) = clean_args.get(0) {
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

            // EXTERNE COMMANDO'S
            let mut command = Command::new(&cmd);
            if !clean_args.is_empty() {
                command.args(&clean_args);
            }

            if let Some(mut prev) = previous_command.take() {
                if let Some(stdout) = prev.stdout.take() {
                    command.stdin(Stdio::from(stdout));
                }
            }

            // Redirection of Pipe?
            if let Some(file_path) = redirect_out {
                let file = OpenOptions::new()
                    .write(true)
                    .create(true)
                    .append(append_out)
                    .truncate(!append_out)
                    .open(&file_path);
                    
                match file {
                    Ok(f) => { command.stdout(Stdio::from(f)); },
                    Err(e) => { eprintln!("omni: kan {} niet openen: {}", file_path, e); continue; }
                }
            } else if i < cmd_count - 1 {
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
