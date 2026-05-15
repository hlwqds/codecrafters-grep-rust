use std::env;
use std::io;
use std::process;

enum PatternType {
    Digit,
    Word,
    CharGroup(String),
    NegCharGroup(String),
    Literal(String),
}

struct Pattern {
    p_type: PatternType,
}

impl Pattern {
    fn new(pattern: String) -> Self {
        if pattern.starts_with("\\d") {
            Pattern {
                p_type: PatternType::Digit,
            }
        } else if pattern.starts_with("\\w") {
            Pattern {
                p_type: PatternType::Word,
            }
        } else if pattern.starts_with("[") && pattern.ends_with("]") {
            if pattern[1..].starts_with("^") {
                Pattern {
                    p_type: PatternType::NegCharGroup(pattern[2..pattern.len() - 1].to_string()),
                }
            } else {
                Pattern {
                    p_type: PatternType::CharGroup(pattern[1..pattern.len() - 1].to_string()),
                }
            }
        } else {
            Pattern {
                p_type: PatternType::Literal(pattern.to_string()),
            }
        }
    }
}

fn match_digit(input_line: &str) -> Option<usize> {
    input_line.find(|c: char| c.is_ascii_digit())
}

fn match_word(input_line: &str) -> Option<usize> {
    input_line.find(|c: char| c.is_ascii_alphanumeric() || c == '_')
}

fn match_char_group(input_line: &str, group: &str) -> Option<usize> {
    input_line.find(|c| group.contains(c))
}

fn match_neg_char_group(input_line: &str, group: &str) -> Option<usize> {
    input_line.find(|c| !group.contains(c))
}

fn match_literal(input_line: &str, literal: &str) -> Option<usize> {
    input_line.find(literal)
}

fn match_pattern(input_line: &str, pattern: &Pattern, anchored: bool) -> Option<usize> {
    let (pos, len) = match &pattern.p_type {
        PatternType::Digit => (match_digit(input_line)?, 1),
        PatternType::Word => (match_word(input_line)?, 1),
        PatternType::CharGroup(group) => (match_char_group(input_line, group)?, 1),
        PatternType::NegCharGroup(group) => (match_neg_char_group(input_line, group)?, 1),
        PatternType::Literal(literal) => (match_literal(input_line, literal)?, literal.len()),
    };
    if anchored && pos != 0 {
        return None;
    }
    Some(pos + len)
}

fn split_patterns(pattern: &str) -> Vec<String> {
    let mut res: Vec<String> = vec![];
    let mut chars = pattern.chars().peekable();
    while let Some(c) = chars.next() {
        let mut p = String::new();
        p.push(c);
        if c == '\\' {
            if let Some(&nc) = chars.peek() {
                p.push(nc);
                chars.next();
            }
        } else if c == '[' {
            while let Some(&nc) = chars.peek() {
                p.push(chars.next().unwrap());
                if nc == ']' {
                    break;
                }
            }
        } else {
            while let Some(&nc) = chars.peek() {
                if nc == '\\' || nc == '[' {
                    break;
                }
                p.push(chars.next().unwrap());
            }
        }
        res.push(p);
    }
    res
}

fn generate_patterns(patterns: Vec<String>) -> Vec<Pattern> {
    let mut res: Vec<Pattern> = vec![];
    for pattern in patterns {
        res.push(Pattern::new(pattern));
    }
    res
}

// Usage: echo <input_text> | your_program.sh -E <pattern>
fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    eprintln!("Logs from your program will appear here!");

    if env::args().nth(1).unwrap() != "-E" {
        println!("Expected first argument to be '-E'");
        process::exit(1);
    }

    let pattern = env::args().nth(2).unwrap();
    let mut input_line = String::new();

    io::stdin().read_line(&mut input_line).unwrap();

    let anchored = pattern.starts_with('^');
    let pattern = pattern.trim_start_matches('^');

    let patterns = generate_patterns(split_patterns(pattern));

    let mut step: usize = 0;

    for (i, pattern) in patterns.into_iter().enumerate() {
        let anc = anchored && i == 0;
        if let Some(s_step) = match_pattern(&input_line[step..], &pattern, anc) {
            step += s_step
        } else {
            process::exit(1)
        }
    }
    process::exit(0)
}
