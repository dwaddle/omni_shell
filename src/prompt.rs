use reedline::{Prompt, PromptHistorySearch, PromptEditMode};
use std::borrow::Cow;
use std::env;
use std::fs;

fn get_fast_git_branch() -> Option<String> {
    if let Ok(head_content) = fs::read_to_string(".git/HEAD") {
        if let Some(branch_path) = head_content.strip_prefix("ref: refs/heads/") {
            return Some(branch_path.trim().to_string());
        } else {
            return Some(head_content.trim()[..7].to_string());
        }
    }
    None
}

#[derive(Clone)]
pub struct OmniPrompt;

impl Prompt for OmniPrompt {
    fn render_prompt_left(&self) -> Cow<str> {
        let current_dir = env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("/"));
        let mut dir_str = current_dir.to_string_lossy().to_string();
        if let Ok(home) = env::var("HOME") {
            if dir_str.starts_with(&home) {
                dir_str = dir_str.replacen(&home, "~", 1);
            }
        }

        let mut git_branch = String::new();
        if let Some(branch) = get_fast_git_branch() {
            git_branch = format!(" \x1b[1;33mgit:({})\x1b[0m", branch);
        }

        Cow::Owned(format!("\x1b[1;34m{}\x1b[0m{}\n", dir_str, git_branch))
    }

    fn render_prompt_right(&self) -> Cow<str> {
        Cow::Borrowed("")
    }

    fn render_prompt_indicator(&self, mode: PromptEditMode) -> Cow<str> {
        match mode {
            PromptEditMode::Vi(reedline::PromptViMode::Normal) => Cow::Borrowed("\x1b[1;31m[N]\x1b[0m omni> "),
            PromptEditMode::Vi(reedline::PromptViMode::Insert) => Cow::Borrowed("\x1b[1;32momni>\x1b[0m "),
            PromptEditMode::Custom(s) => Cow::Owned(s),
            _ => Cow::Borrowed("[1;32momni>[0m "),
        }
    }

    fn render_prompt_multiline_indicator(&self) -> Cow<str> {
        Cow::Borrowed("::: ")
    }

    fn render_prompt_history_search_indicator(&self, _history_search: PromptHistorySearch) -> Cow<str> {
        Cow::Borrowed("(reverse-i-search): ")
    }
}
