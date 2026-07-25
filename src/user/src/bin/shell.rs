#![no_std]
#![no_main]

use user::{entry, println, print};
use user::stdlib::syscalls::{sys_readline, fork, exec, wait};

// \REVIEW: There are several ways to proceed with it.  One way would be to
// simply give out more stack pages to programs.  Another way can have us
// implement C-style strings in Rust so an array of u8 with a start pointer and
// an offset basically.  Any path that you take, just know that these values are
// yours to tweak to perfection:
const MAX_ARGS: usize = 16;
const MAX_COMMANDS: usize = 8;

const INPUT_SIZE: usize = 128;

// spans are stored as u8 offsets into the input line, so the line can't be
// longer than u8::MAX
const _: () = assert!(INPUT_SIZE <= u8::MAX as usize);

// a lightweight (start, len) pair into the input buffer, instead of a fat
// &str (ptr + len), so argv doesn't cost 16 bytes per slot
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    start: u8,
    len: u8,
}

impl Span {
    pub const fn empty() -> Self {
        Self { start: 0, len: 0 }
    }

    pub fn as_str<'a>(&self, input: &'a str) -> &'a str {
        let start = self.start as usize;
        &input[start..start + self.len as usize]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Token {
    Word(Span),
    Pipe,
    Semicolon,
    Background,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LexError {
    UnterminatedString,
}

pub struct Lexer<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            pos: 0
        }
    }

    pub fn next_token(&mut self) -> Result<Option<Token>, LexError> {
        let bytes = self.input.as_bytes();

        // trim whitespaces
        while self.pos < bytes.len() && bytes[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }

        if self.pos >= bytes.len() {
            return Ok(None);
        }

        match bytes[self.pos] {
            b'|' => {
                self.pos += 1;
                Ok(Some(Token::Pipe))
            }

            b';' => {
                self.pos += 1;
                Ok(Some(Token::Semicolon))
            }

            b'&' => {
                self.pos += 1;
                Ok(Some(Token::Background))
            }

            b'"' => {
                self.pos += 1;
                let start = self.pos;
                while self.pos < bytes.len() && bytes[self.pos] != b'"' {
                    self.pos += 1;
                }

                if self.pos == bytes.len() {
                    return Err(LexError::UnterminatedString);
                }

                let span = Span { start: start as u8, len: (self.pos - start) as u8 };
                self.pos += 1;

                Ok(Some(Token::Word(span)))
            }

            _ => {
                let start = self.pos;
                while self.pos < bytes.len() {
                    match bytes[self.pos] {
                        b'|' | b'"' | b';' | b'&' => break,
                        c if c.is_ascii_whitespace() => break,
                        _ => self.pos += 1,
                    }
                }

                let span = Span { start: start as u8, len: (self.pos - start) as u8 };
                Ok(Some(Token::Word(span)))
            }
        }
    }
}

struct Command {
    argc: usize,
    argv: [Span; MAX_ARGS],
}

pub struct ExecutionPipeline {
    count: usize,
    commands: [Command; MAX_COMMANDS],
}

impl Command {
    pub const fn new() -> Self {
        Self {
            argc: 0,
            argv: [Span::empty(); MAX_ARGS],
        }
    }
}

impl ExecutionPipeline {
    pub const fn new() -> Self {
        Self {
            count: 0,
            commands: [const { Command::new() }; MAX_COMMANDS],
        }
    }
}

#[derive(Debug)]
pub enum ParserError {
    Lex(LexError),
    EmptyCommand,
    TooManyArguments,
    TooManyCommands,
}

pub struct Parser<'a> {
    input: &'a str,
    lexer: Lexer<'a>,
}

impl<'a> Parser<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            lexer: Lexer::new(input),
        }
    }

    pub fn parse_and_run(&mut self) -> Result<(), ParserError> {
        let mut pipeline = ExecutionPipeline::new();
        let mut current = Command::new();

        loop {
            let token = match self.lexer.next_token() {
                Ok(token) => token,
                Err(e) => return Err(ParserError::Lex(e)),
            };

            match token {
                Some(Token::Word(span)) => {
                    if current.argc >= MAX_ARGS {
                        return Err(ParserError::TooManyArguments);
                    }

                    current.argv[current.argc] = span;
                    current.argc += 1;
                }

                Some(Token::Pipe) => {
                    if current.argc == 0 {
                        return Err(ParserError::EmptyCommand);
                    }

                    if pipeline.count >= MAX_COMMANDS {
                        return Err(ParserError::TooManyCommands)
                    }

                    pipeline.commands[pipeline.count] = current;
                    pipeline.count += 1;

                    current = Command::new();
                }

                Some(Token::Semicolon) | Some(Token::Background) => {
                    let background = matches!(token, Some(Token::Background));

                    if current.argc == 0 {
                        if pipeline.count > 0 {
                            return Err(ParserError::EmptyCommand);
                        }
                    } else {
                        if pipeline.count >= MAX_COMMANDS {
                            return Err(ParserError::TooManyCommands);
                        }

                        pipeline.commands[pipeline.count] = current;
                        pipeline.count += 1;
                        current = Command::new();
                    }

                    if pipeline.count > 0 {
                        execute_pipeline(&pipeline, background, self.input);
                        pipeline = ExecutionPipeline::new();
                    }
                }

                None => {
                    break;
                }
            }
        }

        if current.argc == 0 {
            if pipeline.count > 0 {
                return Err(ParserError::EmptyCommand);
            }
        } else {
            if pipeline.count >= MAX_COMMANDS {
                return Err(ParserError::TooManyCommands);
            }

            pipeline.commands[pipeline.count] = current;
            pipeline.count += 1;
        }

        if pipeline.count > 0 {
            execute_pipeline(&pipeline, false, self.input);
        }

        Ok(())
    }
}

// @Todo This needs a lot of work after we make the syscall api better.
// That is why execs and forks are not utilised at the moment
fn execute_command(command: &Command, background: bool, input: &str) {
    if command.argc == 0 {
        return;
    }

    let program = command.argv[0].as_str(input);

    if program == "exit" {
        user::stdlib::syscalls::exit(0);
    }

    let mut args: [&str; MAX_ARGS] = [""; MAX_ARGS];
    for i in 1..command.argc {
        args[i - 1] = command.argv[i].as_str(input);
    }
    let args = &args[..command.argc - 1];

    match fork() {
        Ok(0) => {
            // child
            if exec(program, args).is_err() {
                let _ = println!("{}: command not found", program);
            }

            user::stdlib::syscalls::exit(1);
        }

        Ok(pid) => {
            // parent
            if background {
                let _ = println!("[{}] {}", pid, program);
            } else {
                let _ = wait(Some(pid));
            }
        }

        Err(_) => {
            let _ = println!("fork failed!");
        }
    }
}

fn execute_pipeline(pipeline: &ExecutionPipeline, background: bool, input: &str) {
    for i in 0..pipeline.count {
        execute_command(&pipeline.commands[i], background, input);
    }
}

fn main() {
    let mut buf = [0u8; INPUT_SIZE];
    
    loop {
        let _ = print!("$ ");
        let n = sys_readline(&mut buf);

        let input = match core::str::from_utf8(&buf[..n]) {
            Ok(s) => s,
            Err(_) => {
                let _ = println!("invalid utf-8!");
                continue;
            }
        };
        
        let mut parser = Parser::new(input);
        if let Err(e) = parser.parse_and_run() {
            let _ = println!("parser error: {:?}", e);
        }
    }
}

entry!(main);