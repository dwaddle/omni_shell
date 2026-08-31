mod config;
mod executor;
mod completer;
mod prompt;
mod parser;

use reedline::{
    Reedline, Signal, ColumnarMenu, ListMenu, ReedlineMenu, Vi, MenuBuilder, FileBackedHistory,
    default_vi_insert_keybindings, default_vi_normal_keybindings, KeyCode, KeyModifiers, ReedlineEvent,
};
use crate::executor::ShellState;
use crate::prompt::OmniPrompt;
use crate::completer::OmniCompleter;

fn main() {
    let omni_config = config::load_config();
    let mut state = ShellState::new(omni_config);

    // Setup geschiedenis
    let history_path = dirs::home_dir()
        .unwrap()
        .join(".config")
        .join("omni")
        .join("history.txt");
    
    if let Some(p) = history_path.parent() {
        let _ = std::fs::create_dir_all(p);
    }
    
    let history = Box::new(
        FileBackedHistory::with_file(1000, history_path)
            .expect("Kon history bestand niet openen")
    );

    let completion_menu = Box::new(ColumnarMenu::default().with_name("completion_menu"));
    let history_menu = Box::new(ListMenu::default().with_name("history_menu"));
    
    let mut insert_bindings = default_vi_insert_keybindings();
    
    // Tab voor file completion menu
    insert_bindings.add_binding(
        KeyModifiers::NONE,
        KeyCode::Tab,
        ReedlineEvent::UntilFound(vec![
            ReedlineEvent::Menu("completion_menu".to_string()),
            ReedlineEvent::MenuNext,
        ]),
    );
    
    // Ctrl-R voor history menu in plaats van inline reverse search
    insert_bindings.add_binding(
        KeyModifiers::CONTROL,
        KeyCode::Char('r'),
        ReedlineEvent::UntilFound(vec![
            ReedlineEvent::Menu("history_menu".to_string()),
            ReedlineEvent::MenuNext,
        ]),
    );
    
    let mut line_editor = Reedline::create()
        .with_history(history)
        .with_edit_mode(Box::new(Vi::new(insert_bindings, default_vi_normal_keybindings())))
        .with_menu(ReedlineMenu::EngineCompleter(completion_menu))
        .with_menu(ReedlineMenu::HistoryMenu(history_menu))
        .with_completer(Box::new(OmniCompleter));
        
    let prompt = OmniPrompt;

    loop {
        let sig = line_editor.read_line(&prompt);
        match sig {
            Ok(Signal::Success(buffer)) => {
                let input = buffer.trim();
                if input.is_empty() { continue; }
                state.execute_command(input);
            },
            Ok(Signal::CtrlC) => println!("^C"),
            Ok(Signal::CtrlD) => break,
            _ => {}
        }
    }
}
