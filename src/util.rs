
use anyhow::Result;
use base64::{Engine, prelude::BASE64_STANDARD};

pub fn must_read_stdin() -> Result<String> {
    let mut line = String::new();

    std::io::stdin().read_line(&mut line)?;
    line = line.trim().to_owned();
    println!();

    Ok(line)
}

/// encode encodes the input in base64
/// It can optionally zip the input before encoding
pub fn encode(b: &str) -> String {
    //if COMPRESS {
    //    b = zip(b)
    //}

    BASE64_STANDARD.encode(b)
}

/// decode decodes the input from base64
/// It can optionally unzip the input after decoding
pub fn decode(s: &str) -> Result<String> {
    let b = BASE64_STANDARD.decode(s)?;

    //if COMPRESS {
    //    b = unzip(b)
    //}

    let s = String::from_utf8(b)?;
    Ok(s)
}