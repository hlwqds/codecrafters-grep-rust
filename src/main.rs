use std::env;
use std::io;
use std::process;

enum PatternType {
    Digit,
    Word,
    CharGroup(String),
    NegCharGroup(String),
    Literal(String),
    OneOrMore(Box<PatternType>),
    ZeroOrOne(Box<PatternType>),
    WildCard,
}

struct Pattern {
    p_type: PatternType,
}

impl Pattern {
    fn new(pattern: &str) -> Self {
        if pattern.ends_with('+') {
            let inner = &pattern[..pattern.len() - 1];
            let inner_pattern = Pattern::new(inner);
            return Pattern {
                p_type: PatternType::OneOrMore(Box::new(inner_pattern.p_type)),
            };
        } else if pattern.ends_with('?') {
            let inner = &pattern[..pattern.len() - 1];
            let inner_pattern = Pattern::new(inner);
            return Pattern {
                p_type: PatternType::ZeroOrOne(Box::new(inner_pattern.p_type)),
            };
        }
        if pattern == "." {
            return Pattern {
                p_type: PatternType::WildCard,
            };
        }
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

fn match_wildcard(input_line: &str) -> Option<usize> {
    if !input_line.is_empty() {
        Some(0)
    } else {
        None
    }
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

fn char_matches(c: char, p_type: &PatternType) -> bool {
    match p_type {
        PatternType::Digit => c.is_ascii_digit(),
        PatternType::Word => c.is_ascii_alphanumeric() || c == '_',
        PatternType::CharGroup(group) => group.contains(c),
        PatternType::NegCharGroup(group) => !group.contains(c),
        PatternType::Literal(lit) => lit.len() == 1 && lit.as_bytes()[0] == c as u8,
        PatternType::OneOrMore(_) => false,
        &PatternType::ZeroOrOne(_) => false,
        &PatternType::WildCard => true,
    }
}

fn match_single(input_line: &str, pattern: &Pattern) -> Option<(usize, usize)> {
    match &pattern.p_type {
        PatternType::Digit => Some((match_digit(input_line)?, 1)),
        PatternType::Word => Some((match_word(input_line)?, 1)),
        PatternType::CharGroup(group) => Some((match_char_group(input_line, group)?, 1)),
        PatternType::NegCharGroup(group) => Some((match_neg_char_group(input_line, group)?, 1)),
        PatternType::Literal(literal) => Some((match_literal(input_line, literal)?, literal.len())),
        PatternType::OneOrMore(_) => None,
        &PatternType::ZeroOrOne(_) => None,
        &PatternType::WildCard => Some((match_wildcard(input_line)?, 1)),
    }
}

/// Recursively match all patterns, returning total consumed length from input_line.
/// Supports backtracking for OneOrMore.
fn match_patterns(input_line: &str, patterns: &[Pattern], anchored: bool) -> Option<usize> {
    if patterns.is_empty() {
        return Some(0);
    }

    let pattern = &patterns[0];
    let rest = &patterns[1..];

    match &pattern.p_type {
        PatternType::OneOrMore(inner) => {
            let start = input_line.find(|c: char| char_matches(c, inner))?;
            if anchored && start != 0 {
                return None;
            }
            // Try from 1 match upward, return first that lets rest succeed
            let mut end = start;
            for c in input_line[start..].chars() {
                if !char_matches(c, inner) {
                    break;
                }
                end += c.len_utf8();
                if let Some(rest_consumed) = match_patterns(&input_line[end..], rest, true) {
                    return Some(end + rest_consumed);
                }
            }
            None
        }
        PatternType::ZeroOrOne(inner) => {
            // Try 1 match, then 0 matches
            if let Some(c) = input_line.chars().next() {
                if char_matches(c, inner) {
                    let end = c.len_utf8();
                    if let Some(rest_consumed) = match_patterns(&input_line[end..], rest, true) {
                        return Some(end + rest_consumed);
                    }
                }
            }
            // Try matching 0 times
            match_patterns(input_line, rest, true)
        }
        _ => {
            let (pos, len) = match_single(input_line, pattern)?;
            if anchored && pos != 0 {
                return None;
            }
            let end = pos + len;
            match_patterns(&input_line[end..], rest, true).map(|r| end + r)
        }
    }
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
        } else if c == '+' || c == '?' {
            // + modifies the previous token
            if let Some(last) = res.pop() {
                if last.starts_with('\\') || last.starts_with('[') {
                    // \d+ or [abc](+/?) — attach + directly
                    res.push(format!("{}{}", last, c));
                } else if last.len() > 1 {
                    // "aba" + → "ab" + "a(+/?)"
                    let prefix = &last[..last.len() - 1];
                    let last_char = &last[last.len() - 1..];
                    res.push(prefix.to_string());
                    res.push(format!("{}{}", last_char, c));
                } else {
                    // Single char like "a" → "a(+/??"
                    res.push(format!("{}{}", last, c));
                }
            }
            continue;
        } else if c == '.' {
        } else {
            while let Some(&nc) = chars.peek() {
                if nc == '\\' || nc == '[' || nc == '+' || nc == '?' || nc == '*' || nc == '.' {
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
    patterns.into_iter().map(|p| Pattern::new(&p)).collect()
}

// Usage: echo <input_text> | your_program.sh -E <pattern>
fn main() {
    // You can use print statements as following for debugging, they'll be visible when running tests.
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
    let end_anchored = pattern.ends_with('$');
    let pattern = pattern.trim_end_matches('$');

    let patterns = generate_patterns(split_patterns(pattern));

    match match_patterns(&input_line, &patterns, anchored) {
        Some(consumed) if !end_anchored || consumed == input_line.len() => process::exit(0),
        _ => process::exit(1),
    }
}
