use std::io::{self, Write};

// НОВОЕ: верхняя граница для чисел и результатов операций.
// Поменяйте на своё значение, например 999_999, если нужен более строгий лимит.
const LIMIT: i64 = i64::MAX;

fn check(x: i64) -> Result<i64, String> {
    if x > LIMIT || x < -LIMIT {
        Err(format!("Выход за пределы ограничения: {}", x))
    } else {
        Ok(x)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Token {
    Number(i64),
    Plus,
    Minus,
    Multiply,
    Divide,
    LParen,
    RParen,
    Question, // НОВОЕ: '?'
    Colon,    // НОВОЕ: ':'
    EOF,
}

struct Lexer {
    input: Vec<char>,
    pos: usize,
}

impl Lexer {
    fn new(input: &str) -> Self {
        Lexer {
            input: input.chars().collect(),
            pos: 0,
        }
    }

    // НОВОЕ: возвращает Result вместо паники — ошибку удобно показать в REPL
    fn next_token(&mut self) -> Result<Token, String> {
        self.skip_whitespace();
        if self.pos >= self.input.len() {
            return Ok(Token::EOF);
        }
        let ch = self.input[self.pos];
        self.pos += 1;
        match ch {
            '+' => Ok(Token::Plus),
            '-' => Ok(Token::Minus),
            '*' => Ok(Token::Multiply),
            '/' => Ok(Token::Divide),
            '(' => Ok(Token::LParen),
            ')' => Ok(Token::RParen),
            '?' => Ok(Token::Question),
            ':' => Ok(Token::Colon),
            '0'..='9' => {
                let mut num = ch.to_digit(10).unwrap() as i64;
                while self.pos < self.input.len() && self.input[self.pos].is_digit(10) {
                    let d = self.input[self.pos].to_digit(10).unwrap() as i64;
                    // НОВОЕ: переполнение при чтении литерала — ошибка, а не «заворачивание»
                    num = num
                        .checked_mul(10)
                        .and_then(|n| n.checked_add(d))
                        .ok_or_else(|| {
                            format!("Число не помещается в i64 (максимум {})", i64::MAX)
                        })?;
                    self.pos += 1;
                }
                let num = check(num)?;
                Ok(Token::Number(num))
            }
            _ => Err(format!("Неожиданный символ: {}", ch)),
        }
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.input.len() && self.input[self.pos].is_whitespace() {
            self.pos += 1;
        }
    }
}

#[derive(Debug, Clone)]
enum Expr {
    Number(i64),
    BinaryOp {
        left: Box<Expr>,
        op: Token,
        right: Box<Expr>,
    },
    Ternary { // НОВОЕ: узел тернарного оператора
        cond: Box<Expr>,
        then_branch: Box<Expr>,
        else_branch: Box<Expr>,
    },
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, pos: 0 }
    }

    fn current(&self) -> Token {
        self.tokens.get(self.pos).copied().unwrap_or(Token::EOF)
    }

    fn eat(&mut self, expected: Token) {
        if self.current() == expected {
            self.pos += 1;
        } else {
            panic!("Expected {:?}, found {:?}", expected, self.current());
        }
    }

    fn parse(&mut self) -> Expr {
        // НОВОЕ: тернарный оператор — теперь самый низкий уровень приоритета
        self.ternary()
    }

    // НОВОЕ: expr := expr_or '?' expr ':' ternary | expr_or
    // Правая ассоциативность: a ? b : c ? d : e = a ? b : (c ? d : e)
    fn ternary(&mut self) -> Expr {
        let cond = self.expr();
        if self.current() == Token::Question {
            self.eat(Token::Question);
            let then_branch = self.ternary();
            self.eat(Token::Colon);
            let else_branch = self.ternary();
            Expr::Ternary {
                cond: Box::new(cond),
                then_branch: Box::new(then_branch),
                else_branch: Box::new(else_branch),
            }
        } else {
            cond
        }
    }

    fn expr(&mut self) -> Expr {
        let mut node = self.term();
        loop {
            match self.current() {
                Token::Plus => {
                    self.eat(Token::Plus);
                    let right = self.term();
                    node = Expr::BinaryOp {
                        left: Box::new(node),
                        op: Token::Plus,
                        right: Box::new(right),
                    };
                }
                Token::Minus => {
                    self.eat(Token::Minus);
                    let right = self.term();
                    node = Expr::BinaryOp {
                        left: Box::new(node),
                        op: Token::Minus,
                        right: Box::new(right),
                    };
                }
                _ => break,
            }
        }
        node
    }

    fn term(&mut self) -> Expr {
        let mut node = self.factor();
        loop {
            match self.current() {
                Token::Multiply => {
                    self.eat(Token::Multiply);
                    let right = self.factor();
                    node = Expr::BinaryOp {
                        left: Box::new(node),
                        op: Token::Multiply,
                        right: Box::new(right),
                    };
                }
                Token::Divide => {
                    self.eat(Token::Divide);
                    let right = self.factor();
                    node = Expr::BinaryOp {
                        left: Box::new(node),
                        op: Token::Divide,
                        right: Box::new(right),
                    };
                }
                _ => break,
            }
        }
        node
    }

    fn factor(&mut self) -> Expr {
        match self.current() {
            Token::Number(n) => {
                self.eat(Token::Number(n));
                Expr::Number(n)
            }
            Token::LParen => {
                self.eat(Token::LParen);
                let node = self.ternary(); // внутри скобок доступен тернарный оператор
                self.eat(Token::RParen);
                node
            }
            tok => panic!("Unexpected token: {:?}", tok),
        }
    }
}

// НОВОЕ: eval возвращает Result, чтобы ошибки не приводили к панике
fn eval(expr: &Expr) -> Result<i64, String> {
    match expr {
        Expr::Number(n) => Ok(*n),
        Expr::Ternary {
            cond,
            then_branch,
            else_branch,
        } => {
            // Ленивое вычисление: считается только выбранная ветвь
            let c = eval(cond)?;
            if c != 0 {
                eval(then_branch)
            } else {
                eval(else_branch)
            }
        }
        Expr::BinaryOp { left, op, right } => {
            let l = eval(left)?;
            let r = eval(right)?;
            let overflow = || format!("Переполнение при вычислении {} {:?} {}", l, op, r);
            let res = match op {
                Token::Plus => l.checked_add(r).ok_or_else(overflow)?,
                Token::Minus => l.checked_sub(r).ok_or_else(overflow)?,
                Token::Multiply => l.checked_mul(r).ok_or_else(overflow)?,
                Token::Divide => {
                    if r == 0 {
                        return Err("Деление на ноль".to_string());
                    }
                    if l == i64::MIN && r == -1 {
                        return Err("Переполнение при делении (i64::MIN / -1)".to_string());
                    }
                    l / r
                }
                _ => return Err("Недопустимый оператор".to_string()),
            };
            check(res)
        }
    }
}

// НОВОЕ: вынесена лексическая обработка, чтобы ошибки ловить без паник
fn tokenize(input: &str) -> Result<Vec<Token>, String> {
    let mut lexer = Lexer::new(input);
    let mut tokens = Vec::new();
    loop {
        match lexer.next_token() {
            Ok(Token::EOF) => break,
            Ok(tok) => tokens.push(tok),
            Err(e) => return Err(e),
        }
    }
    Ok(tokens)
}

fn main() {
    loop {
        print!("> ");
        io::stdout().flush().unwrap();
        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            println!("Ошибка ввода");
            continue;
        }
        let input = input.trim();
        if input.is_empty() {
            continue;
        }
        if input == "exit" {
            break;
        }
        match tokenize(input) {
            Ok(tokens) => {
                let mut parser = Parser::new(tokens);
                let ast = parser.parse();
                match eval(&ast) {
                    Ok(result) => println!("= {}", result),
                    Err(e) => println!("Ошибка: {}", e),
                }
            }
            Err(e) => println!("Ошибка: {}", e),
        }
    }
}
