use marser::capture;
use marser::{
    error::{AnnotationKind, FurthestFailError, InlineError},
    label::WithLabel,
    matcher::{
        Matcher, MatcherCombinator, commit_on,
        if_error::{if_error, if_error_else_fail},
        many, negative_lookahead,
        none_of::none_of,
        one_or_more, optional,
        parser_matcher::match_parsed,
        positive_lookahead, unwanted,
    },
    one_of::one_of,
    parser::{DeferredWeak, Parser, ParserCombinator, recursive},
};

#[derive(Clone, Debug)]
pub enum UnaryOp {
    Neg,
    Not,
}

#[derive(Clone, Debug)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Less,
    LessOrEqual,
    Equal,
    GreaterOrEqual,
    Greater,
    And,
    Or,
}

#[derive(Clone, Debug)]
pub enum Expr<'src> {
    Num(f64),
    Str(&'src str),
    Bool(bool),
    Var(&'src str),
    UnaryOp {
        operand: Box<Expr<'src>>,
        op: UnaryOp,
    },
    BinOp {
        lhand: Box<Expr<'src>>,
        rhand: Box<Expr<'src>>,
        op: BinOp,
    },
    FuncCall {
        name: &'src str,
        args: Vec<Expr<'src>>,
    },
    Group(Box<Expr<'src>>),
    Invalid(&'src str),
}

#[derive(Clone, Debug)]
pub struct Block<'src> {
    pub statements: Vec<Statement<'src>>,
}

#[derive(Clone, Debug)]
pub enum Statement<'src> {
    Let {
        name: &'src str,
        value: Expr<'src>,
    },
    Assign {
        name: &'src str,
        value: Expr<'src>,
    },
    If {
        condition: Expr<'src>,
        then: Block<'src>,
        else_if: Vec<(Expr<'src>, Block<'src>)>,
        else_block: Option<Block<'src>>,
    },
    While {
        condition: Expr<'src>,
        body: Block<'src>,
    },
    Return(Option<Expr<'src>>),
    Expr(Expr<'src>),
}

#[derive(Clone, Debug)]
pub struct FunctionDef<'src> {
    pub name: &'src str,
    pub params: Vec<&'src str>,
    pub body: Block<'src>,
}

fn whitespace<'src, MRes>() -> impl Matcher<'src, &'src str, MRes> {
    many(one_of((' ', '\t', '\r', '\n')))
}

fn inline_whitespace<'src, MRes>() -> impl Matcher<'src, &'src str, MRes> {
    many(one_of((' ', '\t', '\r')))
}

fn identifier<'src>() -> impl Parser<'src, &'src str, Output = &'src str> {
    capture!((
        bind_slice!(
            (
                one_of(('a'..='z', 'A'..='Z', '_')),
                many(one_of(('a'..='z', 'A'..='Z', '0'..='9', '_'))),
            ),
            slice as &'src str
        ), whitespace())
        => slice
    )
}

fn reserved_word<'src, MRes>() -> impl Matcher<'src, &'src str, MRes> {
    one_of((
        match_parsed(identifier(), "let"),
        match_parsed(identifier(), "if"),
        match_parsed(identifier(), "else"),
        match_parsed(identifier(), "while"),
        match_parsed(identifier(), "true"),
        match_parsed(identifier(), "false"),
        match_parsed(identifier(), "fn"),
        match_parsed(identifier(), "return"),
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

fn number_expr<'src>() -> impl Parser<'src, &'src str, Output = Expr<'src>> {
    capture!(
        commit_on(
            positive_lookahead(one_of(('.', '0'..='9'))),
            bind_slice!((
                many('0'..='9'),
                optional((
                    '.',
                    one_or_more('0'..='9')
                ))
            ), slice as &'src str))
        => slice.parse().unwrap()
    )
    .map_output(Expr::Num)
    .add_error_info(capture!((
        many('0'..='9'),
        '.',
        negative_lookahead('0'..='9')
        )
        => Box::new(|e: &mut FurthestFailError| {
            e.add_note("Numbers must have digits after the decimal point");
        }) as Box<_>
    ))
    .recover_with(capture!(
        bind_slice!(
            many(one_of(('0'..='9', '.'))),
            slice as &'src str
        )
        => Expr::Invalid(slice)
    ))
}

fn string_expr<'src>() -> impl Parser<'src, &'src str, Output = Expr<'src>> {
    capture!(
        commit_on(
            bind_span!('"', open_quote_span as (usize, usize)),
            (
                bind_slice!(
                    many(none_of(('"', '\n'))),
                    slice as &'src str
                ),
                '"'.err_if_no_match(use_binds!(|ctx| {
                    let open_quote_span: Option<(usize, usize)> = open_quote_span.copied();
                    InlineError::new("missing closing quote")
                        .with_span(Some(ctx.span()))
                        .with_annotation(
                            open_quote_span.unwrap(),
                            "quote opened here",
                            AnnotationKind::Context,
                        )
                }))
            )
        )
        => slice
    )
    .map_output(Expr::Str)
}

fn bool_expr<'src>() -> impl Parser<'src, &'src str, Output = Expr<'src>> {
    one_of(("true".to(true), "false".to(false))).map_output(Expr::Bool)
}

fn expr<'src>() -> impl Parser<'src, &'src str, Output = Expr<'src>> {
    recursive(|expr| {
        let group = capture!(commit_on(
            bind_span!('(', open_paren_span as (usize, usize)),
            (
                whitespace(),
                bind!(expr.clone(), expr_inner),
                ')'.err_if_no_match(use_binds!(|ctx| {
                    let open_paren_span: Option<(usize, usize)> = open_paren_span.copied();
                    InlineError::new("missing closing parenthesis")
                        .with_span(Some(ctx.span()))
                        .with_annotation(
                            open_paren_span.unwrap(),
                            "parenthesis opened here",
                            AnnotationKind::Context,
                        )
                }))
            )
        ) => Expr::Group(Box::new(expr_inner)));

        let function_call = capture!(
            commit_on((
                bind!(user_identifier(), name),
                bind_span!('(', open_paren_span as (usize, usize))
            ),(
                optional((
                    bind!(expr.clone(), *args),
                    many((
                        ','.try_insert_if_missing("missing comma between arguments"),
                        inline_whitespace(),
                        if_error(many(unwanted(',', "missing argument"))),
                        inline_whitespace(),
                        bind!(expr.clone(), *args),
                    )),
                    if_error(many(unwanted(',', "trailing comma"))),
                )),
                if_error(many(unwanted(',', "missing argument"))),
                ')'.err_if_no_match(use_binds!(|ctx| {
                    let open_paren_span: Option<(usize, usize)> = open_paren_span.copied();
                    InlineError::new("missing closing parenthesis")
                        .with_span(Some(ctx.span()))
                        .with_annotation(
                            open_paren_span.unwrap(),
                            "bracket opened here",
                            AnnotationKind::Context,
                        )
                }))
            )
            ) => Expr::FuncCall { name, args }
        );

        let var_expr = user_identifier().map_output(Expr::Var);

        let invalid_expr = capture!(
            bind_slice!(
                one_or_more(none_of((
                    whitespace(), '(', ')', '"', '0'..='9', 'a'..='z', 'A'..='Z', '_', '[', ']',
                    one_of(('+', '-', '*', '/', '<', '>', '=', '!', '&', '|', ',', ';'))
                ))),
                slice as &'src str
            )
            => Expr::Invalid(slice)
        )
        .with_label("invalid expression");

        let primary = one_of((
            number_expr(),
            string_expr(),
            bool_expr(),
            function_call,
            var_expr,
            group,
            invalid_expr,
        ));

        let unary = one_of((
            capture!(
                (
                    one_or_more(bind!(one_of(('-'.to(UnaryOp::Neg), '!'.to(UnaryOp::Not))), *ops)),
                    inline_whitespace(),
                    one_of((
                        bind!(primary.clone(), operand),
                        if_error_else_fail(unwanted(
                            bind!(().to(Expr::Invalid("")), operand),
                            "missing operand after unary operator"
                        ))
                    ))
                )
                => ops.into_iter().rev().fold(operand, |acc, op| {
                    Expr::UnaryOp {
                        operand: Box::new(acc),
                        op,
                    }
                })
            ),
            primary.clone(),
        ))
        .erase_types();

        let mul_or_div = capture!(
            (
                bind!(unary.clone(), first_operand),
                whitespace(),
                many((
                    bind!(one_of(('*'.to(BinOp::Mul), '/'.to(BinOp::Div))), *ops),
                    inline_whitespace(),
                    one_of((
                        bind!(unary.clone(), *operands),
                        if_error_else_fail(unwanted(
                            bind!(().to(Expr::Invalid("")), *operands),
                            "missing operand"
                        ))
                    ))
                ))
            )
            => {
                ops.into_iter().zip(operands).fold(first_operand, |acc, (op, operand)| {
                    Expr::BinOp {
                        lhand: Box::new(acc),
                        rhand: Box::new(operand),
                        op,
                    }
                })
            }
        )
        .erase_types();

        let add_or_sub = capture!(
            (
                bind!(mul_or_div.clone(), first_operand),
                whitespace(),
                many((
                    bind!(one_of(('+'.to(BinOp::Add), '-'.to(BinOp::Sub))), *ops),
                    inline_whitespace(),
                    one_of((
                        bind!(mul_or_div.clone(), *operands),
                        if_error_else_fail(
                            unwanted(
                                bind!(().to(Expr::Invalid("")), *operands),
                                "missing operand"
                            )
                        )
                    ))
                ))
            )
            => {
                ops.into_iter().zip(operands).fold(first_operand, |acc, (op, operand)| {
                    Expr::BinOp {
                        lhand: Box::new(acc),
                        rhand: Box::new(operand),
                        op,
                    }
                })
            }
        )
        .erase_types();

        let comparison = capture!(
            (
                bind!(add_or_sub.clone(), first_operand),
                whitespace(),
                many((
                    bind!(one_of((
                        "<=".to(BinOp::LessOrEqual),
                        '<'.to(BinOp::Less),
                        "==".to(BinOp::Equal),
                        ">=".to(BinOp::GreaterOrEqual),
                        '>'.to(BinOp::Greater),
                    )), *ops),
                    inline_whitespace(),
                    one_of((
                        bind!(add_or_sub.clone(), *operands),
                        if_error_else_fail(unwanted(
                            bind!(().to(Expr::Invalid("")), *operands),
                            "missing operand"
                        ))
                    ))
                ))
            )
            => {
                ops.into_iter().zip(operands).fold(first_operand, |acc, (op, operand)| {
                    Expr::BinOp {
                        lhand: Box::new(acc),
                        rhand: Box::new(operand),
                        op,
                    }
                })
            }
        )
        .erase_types();

        let and_expr = capture!(
            (
                bind!(comparison.clone(), first_operand),
                whitespace(),
                many((
                    bind!("&&".to(BinOp::And), *ops),
                    inline_whitespace(),
                    one_of((
                        bind!(comparison.clone(), *operands),
                        if_error_else_fail(unwanted(
                            bind!(().to(Expr::Invalid("")), *operands),
                            "missing operand"
                        ))
                    ))
                ))
            )
            => {
                ops.into_iter().zip(operands).fold(first_operand, |acc, (op, operand)| {
                    Expr::BinOp {
                        lhand: Box::new(acc),
                        rhand: Box::new(operand),
                        op,
                    }
                })
            }
        )
        .erase_types();

        let or_expr = capture!(
            (
                bind!(and_expr.clone(), first_operand),
                whitespace(),
                many((
                    bind!("||".to(BinOp::Or), *ops),
                    inline_whitespace(),
                    one_of((
                        bind!(and_expr.clone(), *operands),
                        if_error_else_fail(unwanted(
                            bind!(().to(Expr::Invalid("")), *operands),
                            "missing operand"
                        ))
                    ))
                ))
            )
            => {
                ops.into_iter().zip(operands).fold(first_operand, |acc, (op, operand)| {
                    Expr::BinOp {
                        lhand: Box::new(acc),
                        rhand: Box::new(operand),
                        op,
                    }
                })
            }
        )
        .erase_types();

        one_of((
            if_error_else_fail(capture!((
                    bind!(or_expr.clone(), valid_expr),
                    many((
                        unwanted((), "missing operand"),
                        or_expr.clone().ignore_result()
                    )),
                ) => valid_expr)),
            or_expr,
        ))
    })
    .erase_types()
}

fn block<'src>() -> impl Parser<'src, &'src str, Output = Block<'src>> {
    recursive(|block: DeferredWeak<_, _>| {
        let let_stmt = capture!(
            commit_on(
                (
                    match_parsed(identifier(), "let"),
                ),
                (
                    bind!(user_identifier(), name),
                    '=',
                    whitespace(),
                    bind!(expr(), value),
                )
            ) => Statement::Let { name, value }
        )
        .add_error_info(one_of((capture!((
                match_parsed(identifier(), "let"),
                negative_lookahead(identifier().ignore_result()),
                bind_span!((), span)
            ) => Box::new(move|e: &mut FurthestFailError| {
                e.add_annotation(
                    span,
                    "missing identifier",
                    AnnotationKind::Context,
                );
            }) as Box<_>),)))
        .erase_types();

        let assign_stmt = capture!(
            (
                bind!(user_identifier(), name),
                '=',
                whitespace(),
                bind!(expr(), value),
            ) => Statement::Assign { name, value }
        )
        .erase_types();

        let if_stmt = capture!(
            commit_on(
                match_parsed(identifier(), "if"),
                (
                    bind!(expr(), condition),
                    bind!(block.clone(), then_block),
                    many((
                        match_parsed(identifier(), "else"),
                        match_parsed(identifier(), "if"),
                        bind!(expr(), *else_if_conditions),
                        bind!(block.clone(), *else_if_blocks),
                    )),
                    optional((
                        match_parsed(identifier(), "else"),
                        bind!(block.clone(), ?else_block),
                    ))
                )
            ) => Statement::If {
                condition,
                then: then_block,
                else_if: else_if_conditions.into_iter().zip(else_if_blocks).collect(),
                else_block,
            }
        )
        .erase_types();

        let while_stmt = capture!(
            commit_on(
                match_parsed(identifier(), "while"),
                (
                    bind!(expr(), condition),
                    bind!(block.clone(), body),
                )
            ) => Statement::While { condition, body }
        )
        .erase_types();

        let return_stmt = capture!(
            (
                match_parsed(identifier(), "return"),
                optional(bind!(expr(), ?value))
            ) => Statement::Return(value)
        )
        .erase_types();

        let expr_stmt = expr().map_output(Statement::Expr);

        let statements_with_semicolons = capture!(
            (
                bind!(one_of((
                    let_stmt,
                    assign_stmt,
                    return_stmt,
                    expr_stmt,
                )), stmt),
                ';'.try_insert_if_missing("missing semicolon after statement"),
                whitespace(),
            ) => stmt
        )
        .erase_types();

        let semicolon_stmt = capture!(
            (
                unwanted(';', "unexpected semicolon"),
                whitespace(),
            ) => Statement::Expr(Expr::Invalid(""))
        )
        .erase_types();

        let statement = one_of((
            if_stmt,
            while_stmt,
            semicolon_stmt,
            statements_with_semicolons,
        ))
        .erase_types();

        capture!(
            commit_on(
                bind_span!('{', open_brace_span as (usize, usize)),
                (
                    whitespace(),
                    many(bind!(statement, *statements)),
                    '}'.err_if_no_match(use_binds!(|ctx| {
                        let open_brace_span: Option<(usize, usize)> = open_brace_span.copied();
                        InlineError::new("missing closing '}'")
                            .with_span(Some(ctx.span()))
                            .with_annotation(
                                open_brace_span.unwrap(),
                                "brace opened here",
                                AnnotationKind::Context,
                            )
                    })),
                    whitespace(),
                )
            ) => Block {
                statements,
            }
        )
        .with_label("block")
        .erase_types()
    })
    .erase_types()
}

fn function_def<'src>() -> impl Parser<'src, &'src str, Output = FunctionDef<'src>> {
    capture!(
        commit_on(
            match_parsed(identifier(), "fn"),
            (
                bind!(user_identifier(), name),
                bind_span!('(', open_paren_span as (usize, usize))
                    .try_insert_if_missing("missing opening '(' in function definition"),
                optional((
                    bind!(user_identifier(), *params),
                    many((
                        ','.try_insert_if_missing("missing comma between parameters"),
                        inline_whitespace(),
                        if_error(many(unwanted(',', "missing parameter"))),
                        inline_whitespace(),
                        bind!(user_identifier(), *params),
                    )),
                    if_error(many(unwanted(',', "trailing comma"))),
                )),
                many(unwanted(',', "missing parameter")),
                ')'.err_if_no_match(use_binds!(|ctx| {
                    let open_paren_span: Option<(usize, usize)> = open_paren_span.copied();
                    InlineError::new("missing closing ')' in function definition")
                        .with_span(Some(ctx.span()))
                        .with_annotation(
                            open_paren_span.unwrap(),
                            "parenthesis opened here",
                            AnnotationKind::Context,
                        )
                })),
                whitespace(),
                bind!(block(), body),
            )
        ) => FunctionDef { name, params, body }
    )
    .with_label("function definition")
    .erase_types()
}

pub fn get_mini_language_grammar<'src>()
-> impl Parser<'src, &'src str, Output = Vec<FunctionDef<'src>>> + Clone {
    capture!(
        (
            whitespace(),
            many((
                bind!(function_def(), *functions),
                whitespace(),
            )),
        ) => functions
    )
    .with_label("file")
    .erase_types()
}
