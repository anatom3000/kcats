#![allow(warnings)]

use std::{env, fs, collections::HashMap, fmt::Display};

type INT_TYPE = i64;

#[derive(Debug, Clone)]
enum Token {
    Ident(String),
    Int(INT_TYPE),
    Str(String),
    LParen,
    RParen,
    Bang,
}

fn resolve_includes(source_path: &str) -> String {
    let source = match fs::read_to_string(source_path) {
        Ok(source) => source,
        Err(e) => panic!("ERROR while reading `{source_path}`: {e:?}")
    };
    let mut included_source = String::new();

    for line in source.lines() {
        if line.starts_with("#include ") {
            included_source.push_str(&resolve_includes(&line[9..]));
        } else {
            included_source.push_str(line);
            included_source.push('\n');
        }
    }

    included_source
}

fn lex(src: String) -> Vec<Token> {
    let mut src = src.chars().peekable();
    let mut tokens = vec![];
    while let Some(c) = src.next() {
        match c {
            '!' if src.peek().copied() != Some('=') => tokens.push(Token::Bang),
            '(' => tokens.push(Token::LParen),
            ')' => tokens.push(Token::RParen),
            '/' if src.peek().copied() == Some('/') => {
                loop {
                    if let Some('\n') | None = src.next() {
                        break;
                    }
                }
            },
            '"' => {
                let mut acc = String::new();
                loop { match src.next() {
                    Some('"') => break,
                    Some(c) => acc.push(c),
                    None => panic!("unfinished string literal"),
                }}

                tokens.push(Token::Str(unquote(acc)));
            }
            '0'..='9' => {
                let mut acc = String::new();
                acc.push(c);
                while let Some(c @ '0'..='9') = src.next() {
                    acc.push(c);
                }

                tokens.push(Token::Int(acc.parse().unwrap()));
            },
            ' ' | '\n' | '\t' => (),
            c => {
                let mut acc = String::new();
                acc.push(c);
                loop { match src.next() {
                    Some(' ' | '\n' | '\t' | '"' | '(' | ')') | None => break,
                    Some(c) => acc.push(c),
                }}
                tokens.push(Token::Ident(acc));
            }
        }
    }

    tokens
}

fn unquote(mut text: String) -> String {
    text.replace("\\n", "\n")
        .replace("\\t", "\t")
}


fn main() {
    let mut args = env::args();
    let _program = args.next().unwrap();
    
    let source = resolve_includes(&args.next().expect("please provide a source file"));
    let tokens = lex(source);

    let mut state = State {
        pc: 0,
        current_stack: 0,
        stacks: vec![Vec::new()],
        labels: HashMap::new(),
    };

    while state.pc < tokens.len() {
        if let Token::Ident(ref instr) = tokens[state.pc] {
            if instr.starts_with(':') {
                state.labels.insert(instr[1..].to_string(), state.pc);
            }
        }
        state.pc += 1;
    }

    state.pc = 0;
    while state.pc < tokens.len() {
        match tokens[state.pc] {
            Token::LParen | Token::RParen => (),
            Token::Bang => i_save_pc(&mut state),
            Token::Int(i) => state.push_int(i),
            Token::Str(ref s) => state.push_string(s.to_string()),
            Token::Ident(ref instr) => match instr.as_str() {
                "+"   => i_add(&mut state),
                "-"   => i_sub(&mut state),
                "*"   => i_mul(&mut state),
                "/"   => i_div(&mut state),
                "%"   => i_mod(&mut state),
                "=="  => i_eq(&mut state),
                "!="  => i_neq(&mut state),
                "=>"  => i_copy_right(&mut state),
                "<="  => i_copy_left(&mut state),
                "."  => i_get(&mut state),
                "->"  => state.switch_to_right_stack(),
                "<-"  => state.switch_to_left_stack(),
                "chr" => i_chr(&mut state),
                "concat" => i_concat(&mut state),
                "debug" => eprintln!("{state}"),
                "dup" => i_dup(&mut state),
                "empty" => i_empty(&mut state),
                "input" => i_input(&mut state),
                "goto_if" => i_goto_if(&mut state),
                "jump" => i_jump(&mut state),
                "jump_if" => i_jump_if(&mut state),
                "len" => i_len(&mut state),
                "pop" => i_pop(&mut state),
                "print" => i_print(&mut state),
                instr if instr.starts_with(':') => (),
                instr => match state.labels.get(instr) {
                    Some(pc) => state.pc = *pc,
                    None => panic!("unknown instruction `{instr}`"),
                }
            }
        }
        state.pc += 1;
    }
}

#[derive(Debug)]
struct State {
    pc: usize,
    current_stack: usize,
    stacks: Vec<Vec<String>>,
    labels: HashMap<String, usize>,
}

impl Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{ pc: {}, current_stack: {}, stacks: {:?} }}",
            self.pc, self.current_stack, self.stacks
        )
    }
}

impl Drop for State {
    fn drop(&mut self) {
        //eprintln!("Exiting state: {self}");
    }
}

impl State {
    fn push_string(&mut self, value: String) {
        self.stacks[self.current_stack].push(value);
    }

    fn push_int(&mut self, value: INT_TYPE) {
        self.stacks[self.current_stack].push(format!("{}", value));
    }

    fn pop_string(&mut self) -> String {
        self.stacks[self.current_stack]
            .pop()
            .expect("failed to pop string from stack")
    }

    fn pop_int(&mut self) -> INT_TYPE {
        self.stacks[self.current_stack]
            .pop()
            .expect("failed to pop int from stack")
            .parse::<INT_TYPE>()
            .expect("failed to convert value to int")
    }

    fn switch_to_left_stack(&mut self) {
        if self.current_stack == 0 {
            panic!("attempted to switch to the left stack from the leftmost stack")
        }

        self.current_stack -= 1;
    }

    fn switch_to_right_stack(&mut self) {
        self.current_stack += 1;

        if self.current_stack == self.stacks.len() {
            self.stacks.push(Vec::new());
        }
    }
}

// INSTRUCTIONS

// $x $y +
fn i_add(state: &mut State) {
    let y = state.pop_int();
    let x = state.pop_int();

    state.push_int(x+y);
}

// $code chr
fn i_chr(state: &mut State) {
    let code = state.pop_int();

    let mut out = String::with_capacity(1);

    out.push(match char::from_u32(code as u32) {
        Some(c) => c,
        None => panic!("`{}` is not a valid character", code),
    });

    state.push_string(out);
}

// $start $end concat
fn i_concat(state: &mut State) {
    let end = state.pop_string();
    let mut start = state.pop_string();

    start.push_str(&end);

    state.push_string(start);
}

// $value <=
fn i_copy_left(state: &mut State) {
    let value = state.pop_string();
    
    state.switch_to_left_stack();
    state.push_string(value);
    state.switch_to_right_stack();
}

// $value =>
fn i_copy_right(state: &mut State) {
    let value = state.pop_string();
    
    state.switch_to_right_stack();
    state.push_string(value);
    state.switch_to_left_stack();
}

// $dividend $divisor /
fn i_div(state: &mut State) {
    let divisor = state.pop_int();
    let dividend = state.pop_int();

    state.push_int(dividend/dividend);
}

// $value dup
fn i_dup(state: &mut State) {
    let value = state.pop_string();

    state.push_string(value.clone());
    state.push_string(value);
}

// empty
fn i_empty(state: &mut State) {
    state.push_int(state.stacks[state.current_stack].is_empty() as INT_TYPE)
}

// input
fn i_input(state: &mut State) {
    use std::io;
    let mut value = String::new();

    io::stdin().read_line(&mut value);
    value.pop();

    state.push_string(value);
}

// $x $y ==
fn i_eq(state: &mut State) {
    let y = state.pop_string();
    let x = state.pop_string();

    state.push_int((x == y) as INT_TYPE);
}

// $string $index .
fn i_get(state: &mut State) {
    let index = state.pop_int();
    let string = state.pop_string();

    let mut c = String::with_capacity(1);
    c.push(match string.chars().nth(index as usize) {
        Some(val) => val,
        None => panic!("attempted to access string {string:?} with an invalid index: {index}"),
    });

    state.push_string(c);
}
 
// $condition $label goto_if
fn i_goto_if(state: &mut State) {
    let label = state.pop_string();
    let condition = state.pop_int();

    if condition != 0 {
        state.pc = match state.labels.get(&label) {
            Some(pc) => *pc,
            None => panic!("unknown label `{label}`"),
        }
    }
}

// $pc jump
fn i_jump(state: &mut State) {
    let pc = state.pop_int();

    state.pc = pc as usize;
}

// $condition $pc jump_if
fn i_jump_if(state: &mut State) {
    let pc = state.pop_int();
    let condition = state.pop_int();

    if condition != 0 {
        state.pc = pc as usize;
    }
}

// $string len
fn i_len(state: &mut State) {
    let string = state.pop_string();

    state.push_int(string.len() as INT_TYPE);
}

// $x $y *
fn i_mul(state: &mut State) {
    let y = state.pop_int();
    let x = state.pop_int();

    state.push_int(x*y);
}

// $value $modulo %
fn i_mod(state: &mut State) {
    let modulo = state.pop_int();
    let value = state.pop_int();

    state.push_int(value % modulo)
}

// $x $y !=
fn i_neq(state: &mut State) {
    let x = state.pop_string();
    let y = state.pop_string();

    state.push_int((x != y) as INT_TYPE);
}

// $value print
fn i_pop(state: &mut State) {
    state.pop_string();
}

// $value print
fn i_print(state: &mut State) {
    use std::io::{self, Write};

    print!("{}", state.pop_string());
    io::stdout().flush().unwrap()
}

// !
fn i_save_pc(state: &mut State) {
    state.push_int(state.pc as INT_TYPE);
}

// $base $by -
fn i_sub(state: &mut State) {
    let by = state.pop_int();
    let base = state.pop_int();

    state.push_int(base-by);
}
