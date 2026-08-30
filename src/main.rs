use std::env;
use std::io::{self, Write};
use std::process::Command;

fn main() {
    loop {
        // 1. Print de prompt
        print!("omni> ");
        io::stdout().flush().unwrap();

        // 2. Lees user input
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        
        let input = input.trim();
        if input.is_empty() {
            continue;
        }

        // 3. Parse het commando (simpele whitespace split)
        let mut parts = input.split_whitespace();
        let command = parts.next().unwrap();
        let args = parts;

        // 4. Ingebouwde commando's (Built-ins)
        match command {
            "exit" => break,
            "cd" => {
                let new_dir = args.peekable().peek().map_or("/", |x| *x);
                let root = std::path::Path::new(new_dir);
                if let Err(e) = env::set_current_dir(&root) {
                    eprintln!("cd fout: {}", e);
                }
            },
            // 5. Externe commando's uitvoeren (fork & exec)
            _ => {
                let mut child = Command::new(command)
                    .args(args)
                    .spawn();

                match child {
                    Ok(mut child) => {
                        let _ = child.wait();
                    },
                    Err(e) => eprintln!("omni: commando niet gevonden: {}", command),
                }
            }
        }
    }
}
