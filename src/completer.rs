use reedline::{Completer, Suggestion, Span, CompletionResult};
use std::path::Path;
use std::fs;

pub struct OmniCompleter;

impl Completer for OmniCompleter {
    fn complete(&mut self, line: &str, pos: usize) -> CompletionResult {
        let mut matches = Vec::new();
        let word_start = line[..pos].rfind(' ').map(|i| i + 1).unwrap_or(0);
        let word = &line[word_start..pos];
        
        let path = Path::new(word);
        let (dir, prefix) = if path.is_dir() && word.ends_with('/') {
            (path, "")
        } else {
            (path.parent().unwrap_or(Path::new(".")), path.file_name().unwrap_or_default().to_str().unwrap_or(""))
        };
        
        let search_dir = if dir.as_os_str().is_empty() { Path::new(".") } else { dir };
        
        if let Ok(entries) = fs::read_dir(search_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                if name.starts_with(prefix) {
                    let mut val = name.clone();
                    if entry.path().is_dir() { val.push('/'); }
                    
                    let replacement = if dir.as_os_str().is_empty() || dir == Path::new(".") {
                        val
                    } else {
                        format!("{}/{}", dir.to_string_lossy(), val)
                    };
                    
                    matches.push(Suggestion {
                        value: replacement,
                        description: None,
                        extra: None,
                        span: Span::new(word_start, pos),
                        append_whitespace: false,
                        display_override: None,
                        style: None,
                        match_indices: None,
                    });
                }
            }
        }
        CompletionResult::fresh(matches)
    }
}
