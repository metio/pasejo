use std::io::{stdin, BufReader, IsTerminal, Read};

use crate::cli::prompts;

pub fn secret(secret_path: &String, multiline: bool) -> anyhow::Result<String> {
    let secret = if stdin().is_terminal() {
        prompts::read_secret_from_user_input(secret_path, multiline)?
    } else {
        let mut str = String::new();
        BufReader::new(stdin().lock()).read_to_string(&mut str)?;
        str
    };
    Ok(secret)
}
