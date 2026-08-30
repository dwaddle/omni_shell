mod parser;
mod config;
mod executor;
mod helper;
mod prompt;

use rustyline::error::ReadlineError;
use rustyline::{Cmd, Config, EditMode, Editor, KeyEvent, Modifiers};
use rustyline::hint::HistoryHinter;
use rustyline::highlight::MatchingBracketHighlighter;
use rustyline::completion::FilenameCompleter;

use crate::executor::ShellState;
use crate::helper::OmniHelper;

fn main() -> rustyline::Result<()> {
    let omni_config = config::load_config();
    let mut state = ShellState::new(omni_config);

    let rl_config = Config::builder().edit_mode(EditMode::Vi).build();
    let h = OmniHelper {
        completer: FilenameCompleter::new(),
        highlighter: MatchingBracketHighlighter::new(),
        hinter: HistoryHinter::new(),
    };
    
    let mut rl = Editor::<OmniHelper, rustyline::history::DefaultHistory>::with_config(rl_config)?;
    rl.set_helper(Some(h));
    rl.bind_sequence(KeyEvent::ctrl('r'), Cmd::ReverseSearchHistory);
    
    let history_path = config::get_history_path();
    let _ = rl.load_history(&history_path);

    loop {
        let prompt_str = prompt::build_prompt();
        let readline = rl.readline(&prompt_str);
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
