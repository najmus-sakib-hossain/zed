#![no_main]

use libfuzzer_sys::fuzz_target;
use serializer::tokenizer::{Token, Tokenizer};

fuzz_target!(|data: &[u8]| {
    // Fuzz the tokenizer directly
    // The tokenizer should never panic on any input
    let mut tokenizer = Tokenizer::new(data);
    
    // Consume all tokens until EOF or error
    let mut count = 0;
    loop {
        match tokenizer.next_token() {
            Ok(Token::Eof) => break,
            Ok(_) => {
                count += 1;
                // Prevent infinite loops
                if count > 100_000 {
                    break;
                }
            }
            Err(_) => break,
        }
    }
});
