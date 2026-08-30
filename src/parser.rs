#[derive(Debug, Clone, PartialEq)]
pub enum LogicOp {
    And,
    Or,
    Semi,
    None,
}

#[derive(Debug, Clone)]
pub struct Pipeline {
    pub commands: Vec<String>,
    pub op: LogicOp,
}

pub fn parse_pipelines(input: &str) -> Vec<Pipeline> {
    let mut pipelines = Vec::new();
    let mut current_pipeline = Vec::new();
    let mut current_cmd = String::new();

    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];

        if c == '\'' && !in_double_quote {
            in_single_quote = !in_single_quote;
        } else if c == '"' && !in_single_quote {
            in_double_quote = !in_double_quote;
        }

        let in_quotes = in_single_quote || in_double_quote;

        if !in_quotes {
            // Check for &&
            if c == '&' && i + 1 < chars.len() && chars[i + 1] == '&' {
                current_pipeline.push(current_cmd.trim().to_string());
                current_cmd.clear();
                pipelines.push(Pipeline {
                    commands: current_pipeline.clone(),
                    op: LogicOp::And,
                });
                current_pipeline.clear();
                i += 2;
                continue;
            }
            // Check for ||
            if c == '|' && i + 1 < chars.len() && chars[i + 1] == '|' {
                current_pipeline.push(current_cmd.trim().to_string());
                current_cmd.clear();
                pipelines.push(Pipeline {
                    commands: current_pipeline.clone(),
                    op: LogicOp::Or,
                });
                current_pipeline.clear();
                i += 2;
                continue;
            }
            // Check for ;
            if c == ';' {
                current_pipeline.push(current_cmd.trim().to_string());
                current_cmd.clear();
                pipelines.push(Pipeline {
                    commands: current_pipeline.clone(),
                    op: LogicOp::Semi,
                });
                current_pipeline.clear();
                i += 1;
                continue;
            }
            // Check for |
            if c == '|' {
                current_pipeline.push(current_cmd.trim().to_string());
                current_cmd.clear();
                i += 1;
                continue;
            }
        }

        current_cmd.push(c);
        i += 1;
    }

    if !current_cmd.trim().is_empty() {
        current_pipeline.push(current_cmd.trim().to_string());
    }

    if !current_pipeline.is_empty() {
        pipelines.push(Pipeline {
            commands: current_pipeline,
            op: LogicOp::None,
        });
    }

    pipelines
}
