use rustyline::error::ReadlineError;
use rustyline::{Config, EditMode};
use std::env;
use std::process::Command;

fn main() -> rustyline::Result<()> {
    // Configureer de shell voor Vi (Vim) editing mode
    let config = Config::builder().edit_mode(EditMode::Vi).build();

    let mut rl = rustyline::Editor::<(), rustyline::history::DefaultHistory>::with_config(config)?;

    // Probeer eventuele shell history in te laden
    if rl.load_history("history.txt").is_err() {
        println!("Geen eerdere geschiedenis gevonden.");
    }

    loop {
        // Lees input met de geconfigureerde rustyline prompt
        let readline = rl.readline("omni> ");

        match readline {
            Ok(line) => {
                let input = line.trim();
                if input.is_empty() {
                    continue;
                }

                // Voeg toe aan geschiedenis
                let _ = rl.add_history_entry(input);

                let mut parts = input.split_whitespace();
                let command = parts.next().unwrap();
                let args = parts;

                match command {
                    "exit" => break,
                    "cd" => {
                        let new_dir = args.peekable().peek().map_or("/", |x| *x);
                        let root = std::path::Path::new(new_dir);
                        if let Err(e) = env::set_current_dir(&root) {
                            eprintln!("cd fout: {}", e);
                        }
                    }
                    _ => {
                        let child = Command::new(command).args(args).spawn();

                        match child {
                            Ok(mut child) => {
                                let _ = child.wait();
                            }
                            Err(_) => eprintln!("omni: commando niet gevonden: {}", command),
                        }
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                // Ctrl-C
                println!("^C");
            }
            Err(ReadlineError::Eof) => {
                // Ctrl-D
                println!("exit");
                break;
            }
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
    }

    // Bewaar geschiedenis voor een volgende sessie
    rl.save_history("history.txt")?;
    Ok(())
}
