// A small scripting-language parser built with `marser`.
//
// The language supports variable declarations, assignments, print statements,
// blocks, if/else, while loops, boolean literals, numeric literals, identifiers,
// unary operators, binary operators with precedence, and parenthesized expressions.
//
// Example input:
//
//     let x = 1 + 2 * 3;
//     print x;
//
//     if x > 3 {
//         print x;
//     } else {
//         print 0;
//     }
//
// The file is organized in layers:
//
// 1. AST types (`Program`, `Stmt`, `Expr`, operators).
// 2. Small token-like parsers (`identifier`, `number_parser`, whitespace).
// 3. Expression parsing, from primary expressions up through precedence levels.
// 4. Statement and block parsing, including a few recovery examples.
// 5. A tiny CLI that parses a file and prints the resulting AST.
//
// Most parsers use `capture!`: the matcher part describes what to consume, and
// the expression after `=>` builds the AST value from any bound pieces.
use std::{env, fs, process, rc::Rc};

use marser::{
    error::{FurthestFailError, ParserError}, input::Input, label::WithLabel, matcher::{
        AnyToken, Matcher, MatcherCombinator, commit_matcher::commit_on, if_error::if_error, multiple::many, negative_lookahead::negative_lookahead, one_or_more::one_or_more, optional::optional, parser_matcher::match_parsed, positive_lookahead::positive_lookahead, unwanted::unwanted
    }, one_of::one_of, parser::{Parser, ParserCombinator, deferred::recursive}
};
use marser_macros::capture;

type Span = (usize, usize);

fn whitespace<'src, Inp, MRes>() -> impl Matcher<'src, Inp, MRes>
where
    Inp: Input<'src, Token = char>,
{
    many(one_of((' ', '\t', '\n', '\r')))
}

#[derive(Debug, Clone, PartialEq)]
pub struct Program<'src> {
    pub statements: Vec<Stmt<'src>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block<'src> {
    pub statements: Vec<Stmt<'src>>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Ident<'src> {
    pub name: &'src str,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Negate,
    Not,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Add,
    Subtract,
    Multiply,
    Divide,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct UnaryToken {
    op: UnaryOp,
    span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt<'src> {
    Let { name: Ident<'src>, value: Expr<'src> },
    Assign { name: Ident<'src>, value: Expr<'src> },
    Print(Expr<'src>),
    If {
        condition: Expr<'src>,
        then_block: Block<'src>,
        else_block: Option<Block<'src>>,
    },
    While {
        condition: Expr<'src>,
        body: Block<'src>,
    },
    Block(Block<'src>),
    Expr(Expr<'src>),
    Invalid(&'src str),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr<'src> {
    Number { literal: &'src str, span: Span },
    Bool { value: bool, span: Span },
    Ident(Ident<'src>),
    Group { expr: Box<Expr<'src>>, span: Span },
    Unary {
        op: UnaryOp,
        expr: Box<Expr<'src>>,
        span: Span,
    },
    Binary {
        left: Box<Expr<'src>>,
        op: BinaryOp,
        right: Box<Expr<'src>>,
        span: Span,
    },
    Invalid { source: &'src str, span: Span },
}

impl<'src> Expr<'src> {
    fn span(&self) -> Span {
        match self {
            Expr::Number { span, .. }
            | Expr::Bool { span, .. }
            | Expr::Group { span, .. }
            | Expr::Unary { span, .. }
            | Expr::Binary { span, .. }
            | Expr::Invalid { span, .. } => *span,
            Expr::Ident(ident) => ident.span,
        }
    }
}

fn merge_span(left: Span, right: Span) -> Span {
    (left.0, right.1)
}

// Each binary precedence level parses a flat shape:
//
//     first op right op right ...
//
// `fold_binary` turns that into a left-associative AST. For example,
// `a - b - c` becomes `(a - b) - c`.
fn fold_binary<'src>(
    first: Expr<'src>,
    ops: Vec<BinaryOp>,
    rights: Vec<Expr<'src>>,
) -> Expr<'src> {
    ops.into_iter()
        .zip(rights)
        .fold(first, |left, (op, right)| {
            let span = merge_span(left.span(), right.span());
            Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
                span,
            }
        })
}

fn identifier<'src>() -> impl Parser<'src, &'src str, Output = &'src str> {
    // `bind_slice!` returns a borrowed slice from the original source text, so
    // identifiers can be stored in the AST without allocating new strings.
    capture!(
        (
            bind_slice!(
                (
                    one_of(('a'..='z', 'A'..='Z', '_')),
                    many(one_of(('a'..='z', 'A'..='Z', '0'..='9', '_'))),
                ),
                name as &'src str
            ),
            whitespace(),
        ) => name
    )
    .with_label("identifier")
}

fn reserved_word<'src, MRes>() -> impl Matcher<'src, &'src str, MRes> {
    one_of((
        match_parsed(identifier(), "let"),
        match_parsed(identifier(), "print"),
        match_parsed(identifier(), "if"),
        match_parsed(identifier(), "else"),
        match_parsed(identifier(), "while"),
        match_parsed(identifier(), "true"),
        match_parsed(identifier(), "false"),
    ))
}

fn user_identifier<'src>() -> impl Parser<'src, &'src str, Output = &'src str> {
    capture!(
        (
            negative_lookahead(reserved_word()),
            bind!(identifier(), name),
        ) => name
    )
    .with_label("identifier")
}

fn number_text_parser<'src>() -> impl Parser<'src, &'src str, Output = &'src str> {
    capture!(
        bind_slice!(
            (
                one_or_more('0'..='9'),
                optional(('.', one_or_more('0'..='9'))),
            ),
            literal as &'src str
        ) => literal
    )
}

fn number_parser<'src>() -> impl Parser<'src, &'src str, Output = Expr<'src>> {
    capture!(
        (
            bind!(number_text_parser(), literal, span as Span),
            negative_lookahead(one_of(('a'..='z', 'A'..='Z', '0'..='9', '_'))),
            whitespace(),
        ) => Expr::Number { literal, span }
    )
    .with_label("number")
}

fn invalid_expr_parser<'src>() -> impl Parser<'src, &'src str, Output = Expr<'src>> {
    capture!(
        (
            bind!(
                capture!(
                    bind_slice!(
                        many((
                            negative_lookahead(one_of((';', ')', '}'))),
                            AnyToken,
                        )),
                        source as &'src str
                    ) => source
                ),
                source,
                span as Span
            ),
            whitespace(),
        ) => Expr::Invalid { source, span }
    )
}

fn missing_expr_parser<'src>() -> impl Parser<'src, &'src str, Output = Expr<'src>> {
    capture!(
        bind_span!(
            unwanted(
                positive_lookahead(one_of((';', ')', '}', '{'))),
                "missing expression",
            ),
            span as Span
        ) => Expr::Invalid {
            source: "",
            span,
        }
    )
}

fn expression_parser<'src>() -> impl Parser<'src, &'src str, Output = Expr<'src>> {
    // Expressions are parsed in layers, one precedence level at a time:
    //
    // primary    literals, identifiers, and parenthesized expressions
    // unary      !expr and -expr
    // factor     * and /
    // term       + and -
    // comparison < <= > >=
    // equality   == !=
    //
    // Lower layers are bound by higher layers, which gives `1 + 2 * 3` the AST
    // shape `1 + (2 * 3)`.
    recursive(|expr| {
        let ident = Rc::new(user_identifier());
        let number = Rc::new(number_parser());
        let invalid_expr = Rc::new(invalid_expr_parser());
        let missing_expr = Rc::new(missing_expr_parser());

        let true_lit = capture!(
            bind_span!(match_parsed(identifier(), "true"), span as Span)
                => Expr::Bool { value: true, span }
        )
        .with_label("true");

        let false_lit = capture!(
            bind_span!(match_parsed(identifier(), "false"), span as Span)
                => Expr::Bool { value: false, span }
        )
        .with_label("false");

        let grouped = capture!(
                commit_on(
                    ('(', whitespace()),
                    (
                        bind!(expr.clone(), inner),
                        ')'.try_insert_if_missing("missing closing ')'"),
                        whitespace(),
                    )
                ) => {
                let inner: Expr<'src> = inner;
                Expr::Group {
                span: inner.span(),
                expr: Box::new(inner),
            }
            }
        )
        .recover_with(invalid_expr.clone())
        .with_label("grouped expression");

        let ident_expr = capture!(
            bind!(ident.clone(), name, span as Span) => Expr::Ident(Ident { name, span })
        );

        let primary = Rc::new(one_of((
            grouped,
            number.clone(),
            true_lit,
            false_lit,
            ident_expr,
        )));

        let unary = Rc::new(recursive(|unary| {
            let unary_op = one_of((
                capture!((bind_span!('!', span as Span), whitespace()) => UnaryToken {
                    op: UnaryOp::Not,
                    span,
                }),
                capture!((bind_span!('-', span as Span), whitespace()) => UnaryToken {
                    op: UnaryOp::Negate,
                    span,
                }),
            ));
            one_of((
                capture!(
                    (
                        bind!(unary_op, op),
                        bind!(one_of((unary.clone(), missing_expr.clone())), expr),
                    ) => {
                    let op: UnaryToken = op;
                    let expr: Expr<'src> = expr;
                    Expr::Unary {
                        span: merge_span(op.span, expr.span()),
                        op: op.op,
                        expr: Box::new(expr),
                    }
                }
                ),
                primary.clone(),
            ))
        }));

        let factor = Rc::new(capture!(
            (
                bind!(unary.clone(), first),
                many((
                    bind!(
                        one_of((
                            '*'.to(BinaryOp::Multiply),
                            '/'.to(BinaryOp::Divide),
                        )),
                        *ops
                    ),
                    whitespace(),
                    bind!(one_of((unary.clone(), missing_expr.clone())), *rights),
                )),
            ) => fold_binary(first, ops, rights)
        ));

        let term = Rc::new(capture!(
            (
                bind!(factor.clone(), first),
                many((
                    bind!(
                        one_of((
                            '+'.to(BinaryOp::Add),
                            '-'.to(BinaryOp::Subtract),
                        )),
                        *ops
                    ),
                    whitespace(),
                    bind!(one_of((factor.clone(), missing_expr.clone())), *rights),
                )),
            ) => fold_binary(first, ops, rights)
        ));

        let comparison = Rc::new(capture!(
            (
                bind!(term.clone(), first),
                many((
                    bind!(
                        one_of((
                            "<=".to(BinaryOp::LessEqual),
                            ">=".to(BinaryOp::GreaterEqual),
                            '<'.to(BinaryOp::Less),
                            '>'.to(BinaryOp::Greater),
                        )),
                        *ops
                    ),
                    whitespace(),
                    bind!(one_of((term.clone(), missing_expr.clone())), *rights),
                )),
            ) => fold_binary(first, ops, rights)
        ));

        capture!(
            (
                bind!(comparison.clone(), first),
                many((
                    bind!(
                        one_of((
                            "==".to(BinaryOp::Equal),
                            "!=".to(BinaryOp::NotEqual),
                        )),
                        *ops
                    ),
                    whitespace(),
                    bind!(one_of((comparison.clone(), missing_expr.clone())), *rights),
                )),
            ) => fold_binary(first, ops, rights)
        )
        .with_label("expression")
    })
    .maybe_erase_types()
}

fn invalid_statement_parser<'src>() -> impl Parser<'src, &'src str, Output = Stmt<'src>> {
    capture!(
        (
            bind_slice!(
                unwanted(
                    one_or_more((
                        negative_lookahead(one_of((';', '}'))),
                        AnyToken,
                    )),
                    "invalid statement",
                ),
                source as &'src str
            ),
            optional(';'),
            whitespace(),
        ) => Stmt::Invalid(source)
    )
}

pub fn get_mini_script_grammar<'src>() -> impl Parser<'src, &'src str, Output = Program<'src>> {
    // Statements are recursive because blocks contain statements, and statements
    // can themselves be blocks.
    let statement = recursive(|statement| {
        let expr = Rc::new(expression_parser());
        let required_expr = Rc::new(one_of((expr.clone(), missing_expr_parser())));
        let ident = Rc::new(user_identifier());
        let invalid_statement = Rc::new(invalid_statement_parser());

        let block = Rc::new(capture!(
                commit_on(
                    ('{', whitespace()),
                    (
                        many(bind!(statement.clone(), *statements)),
                        if_error(many((
                            unwanted(';', "extra semicolon"),
                            whitespace(),
                        ))),
                        '}'.try_insert_if_missing("missing closing '}'"),
                        whitespace(),
                    )
                ) => Block {
                    statements,
                    span: (0, 0),
                }
        ));

        let let_stmt = capture!(
            commit_on(
                (
                    match_parsed(identifier(), "let"),
                    bind!(ident.clone(), name, name_span as Span),
                ),
                (
                    '='.try_insert_if_missing("missing '=' in let statement"),
                    whitespace(),
                    bind!(required_expr.clone(), value),
                    ';'.try_insert_if_missing("missing semicolon after let statement"),
                    whitespace(),
                )
            ) => Stmt::Let {
                name: Ident {
                    name,
                    span: name_span,
                },
                value,
            }
        )
        // Once a statement has committed to being a `let` statement, recovery
        // can produce an invalid statement node instead of aborting the whole parse.
        .recover_with(invalid_statement.clone())
        .with_label("let statement");

        let print_stmt = capture!(
            commit_on(
                match_parsed(identifier(), "print"),
                (
                    bind!(required_expr.clone(), value),
                    ';'.try_insert_if_missing("missing semicolon after print statement"),
                    whitespace(),
                )
            ) => Stmt::Print(value)
        )
        .recover_with(invalid_statement.clone())
        .with_label("print statement");

        let if_stmt = capture!(
            commit_on(
                match_parsed(identifier(), "if"),
                (
                    bind!(required_expr.clone(), condition),
                    bind!(block.clone(), then_block),
                    optional((
                        match_parsed(identifier(), "else"),
                        bind!(block.clone(), ?else_block),
                    )),
                )
            ) => Stmt::If {
                condition,
                then_block,
                else_block,
            }
        )
        .recover_with(invalid_statement.clone())
        .with_label("if statement");

        let while_stmt = capture!(
            commit_on(
                match_parsed(identifier(), "while"),
                (
                    bind!(required_expr.clone(), condition),
                    bind!(block.clone(), body),
                )
            ) => Stmt::While { condition, body }
        )
        .recover_with(invalid_statement.clone())
        .with_label("while statement");

        let block_stmt = capture!(
            bind!(block.clone(), parsed_block) => Stmt::Block(parsed_block)
        );

        let assign_stmt = capture!(
            commit_on(
                (
                    bind!(ident.clone(), name, name_span as Span),
                    positive_lookahead('='),
                    '=',
                    whitespace(),
                ),
                (
                    bind!(required_expr.clone(), value),
                    ';'.try_insert_if_missing("missing semicolon after assignment"),
                    whitespace(),
                )
            ) => Stmt::Assign {
                name: Ident {
                    name,
                    span: name_span,
                },
                value,
            }
        )
        .recover_with(invalid_statement.clone())
        .with_label("assignment");

        let expr_stmt = capture!(
            (
                bind!(expr.clone(), value),
                ';'.try_insert_if_missing("missing semicolon after expression"),
                whitespace(),
            ) => Stmt::Expr(value)
        )
        .recover_with(invalid_statement.clone())
        .with_label("expression statement");

        one_of((
            let_stmt,
            print_stmt,
            if_stmt,
            while_stmt,
            block_stmt,
            assign_stmt,
            expr_stmt,
            invalid_statement.clone(),
        ))
        .with_label("statement")
    })
    .maybe_erase_types();

    capture!(
        (
            whitespace(),
            many(bind!(statement, *statements)),
        ) => Program { statements }
    )
    .maybe_erase_types()
}

fn print_errors(errors: &[ParserError], source_id: &str, source: &str) {
    if errors.is_empty() {
        return;
    }

    eprintln!("recovered with {} diagnostic(s):", errors.len());
    ParserError::eprint_many(errors, source_id, source);
}

fn main() {
    let mut args = env::args();
    let program_name = args.next().unwrap_or_else(|| "mini_script".to_string());
    let Some(path) = args.next() else {
        eprintln!("usage: {program_name} <script.ms>");
        process::exit(2);
    };

    let src = match fs::read_to_string(&path) {
        Ok(src) => src,
        Err(err) => {
            eprintln!("failed to read '{path}': {err}");
            process::exit(1);
        }
    };

    match marser::parse(get_mini_script_grammar(), &src) {
        Ok((program, errors)) => {
            println!("{program:#?}");
            print_errors(&errors, &path, &src);
        }
        Err(err) => {
            err.eprint(&path, &src);
            let parser_error: FurthestFailError = err;
            eprintln!("{parser_error}");
            process::exit(1);
        }
    }
}
