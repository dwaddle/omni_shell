use std::env;
use std::process::{Command, Stdio, Child};
use std::io::Read;
use glob::glob;
use shellexpand;
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

    pub fn execute_command(&mut self, input: &str) {
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
            if cmd == "alias" {
                if final_args.is_empty() {
                    for (k, v) in &self.config.aliases {
                        println!("alias {}='{}'", k, v);
                    }
                } else {
                    let full_arg = final_args.join(" ");
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
