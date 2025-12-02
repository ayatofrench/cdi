pub fn split_command_into_parts(input: &str) -> Option<(String, Vec<String>)> {
    let trimmed_command_str = input.trim();
    if trimmed_command_str.is_empty() {
        return None;
    }

    let mut parts = Vec::new();
    let mut current_part = String::new();
    let mut in_quotes = false;

    for char_code in trimmed_command_str.chars() {
        match char_code {
            '"' => {
                in_quotes = !in_quotes;
            }
            ' ' if !in_quotes => {
                if !current_part.is_empty() {
                    parts.push(current_part.clone());
                    current_part.clear();
                }
            }
            _ => {
                current_part.push(char_code);
            }
        }
    }
    if !current_part.is_empty() {
        parts.push(current_part);
    }

    match parts.first() {
        Some(command) => {
            let args = parts.iter().skip(1).cloned().collect();

            return Some((command.to_string(), args));
        }
        None => None,
    }
}
