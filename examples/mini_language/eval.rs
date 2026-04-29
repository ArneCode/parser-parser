use std::collections::HashMap;
use std::fmt;
use std::io::{self, Write};

use super::grammar::{BinOp, Block, Expr, FunctionDef, Statement, UnaryOp};

#[derive(Clone, Debug)]
pub enum Value {
    Num(f64),
    Str(String),
    Bool(bool),
    Unit,
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Num(v) => write!(f, "{v}"),
            Self::Str(v) => write!(f, "{v}"),
            Self::Bool(v) => write!(f, "{v}"),
            Self::Unit => write!(f, "()"),
        }
    }
}

#[derive(Debug)]
pub struct RuntimeError {
    message: String,
}

impl RuntimeError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "runtime error: {}", self.message)
    }
}

impl std::error::Error for RuntimeError {}

type BuiltinHandler = fn(&[Value]) -> Result<Value, RuntimeError>;

#[derive(Clone, Copy)]
struct Builtin {
    min_arity: usize,
    max_arity: Option<usize>,
    handler: BuiltinHandler,
}

enum ExecFlow {
    Continue,
    Return(Value),
}

struct Runtime<'src> {
    functions: HashMap<&'src str, &'src FunctionDef<'src>>,
    builtins: HashMap<&'static str, Builtin>,
    scopes: Vec<HashMap<&'src str, Value>>,
}

impl<'src> Runtime<'src> {
    fn new(functions: &'src [FunctionDef<'src>]) -> Self {
        let mut function_map = HashMap::new();
        for function in functions {
            function_map.insert(function.name, function);
        }

        let builtins = HashMap::from([
            (
                "print",
                Builtin {
                    min_arity: 1,
                    max_arity: None,
                    handler: builtin_print,
                },
            ),
            (
                "input",
                Builtin {
                    min_arity: 0,
                    max_arity: Some(1),
                    handler: builtin_input,
                },
            ),
            (
                "str_to_num",
                Builtin {
                    min_arity: 1,
                    max_arity: Some(1),
                    handler: builtin_str_to_num,
                },
            ),
            (
                "num_to_str",
                Builtin {
                    min_arity: 1,
                    max_arity: Some(1),
                    handler: builtin_num_to_str,
                },
            ),
        ]);

        Self {
            functions: function_map,
            builtins,
            scopes: vec![HashMap::new()],
        }
    }

    fn run(mut self) -> Result<Value, RuntimeError> {
        let Some(main_fn) = self.functions.get("main").copied() else {
            return Err(RuntimeError::new("entrypoint function `main` not found"));
        };

        if !main_fn.params.is_empty() {
            return Err(RuntimeError::new(
                "entrypoint function `main` must not take parameters",
            ));
        }

        self.call_user_function(main_fn, Vec::new())
    }

    fn call_function(&mut self, name: &'src str, args: Vec<Value>) -> Result<Value, RuntimeError> {
        if let Some(function) = self.functions.get(name).copied() {
            return self.call_user_function(function, args);
        }
        if let Some(builtin) = self.builtins.get(name).copied() {
            return self.call_builtin(name, builtin, args);
        }
        Err(RuntimeError::new(format!("unknown function `{name}`")))
    }

    fn call_user_function(
        &mut self,
        function: &'src FunctionDef<'src>,
        args: Vec<Value>,
    ) -> Result<Value, RuntimeError> {
        if function.params.len() != args.len() {
            return Err(RuntimeError::new(format!(
                "function `{}` expects {} args but got {}",
                function.name,
                function.params.len(),
                args.len()
            )));
        }

        let mut frame = HashMap::new();
        for (name, arg) in function.params.iter().zip(args) {
            frame.insert(*name, arg);
        }
        self.scopes.push(frame);
        let flow = self.exec_block(&function.body)?;
        self.scopes.pop();

        match flow {
            ExecFlow::Continue => Ok(Value::Unit),
            ExecFlow::Return(value) => Ok(value),
        }
    }

    fn call_builtin(
        &self,
        name: &str,
        builtin: Builtin,
        args: Vec<Value>,
    ) -> Result<Value, RuntimeError> {
        if args.len() < builtin.min_arity {
            return Err(RuntimeError::new(format!(
                "builtin `{name}` expects at least {} args but got {}",
                builtin.min_arity,
                args.len()
            )));
        }
        if let Some(max_arity) = builtin.max_arity
            && args.len() > max_arity
        {
            return Err(RuntimeError::new(format!(
                "builtin `{name}` expects at most {max_arity} args but got {}",
                args.len()
            )));
        }
        (builtin.handler)(&args)
    }

    fn exec_block(&mut self, block: &Block<'src>) -> Result<ExecFlow, RuntimeError> {
        self.scopes.push(HashMap::new());
        for statement in &block.statements {
            let flow = self.exec_statement(statement)?;
            if matches!(flow, ExecFlow::Return(_)) {
                self.scopes.pop();
                return Ok(flow);
            }
        }
        self.scopes.pop();
        Ok(ExecFlow::Continue)
    }

    fn exec_statement(&mut self, statement: &Statement<'src>) -> Result<ExecFlow, RuntimeError> {
        match statement {
            Statement::Let { name, value } => {
                let value = self.eval_expr(value)?;
                let scope = self.scopes.last_mut().expect("at least one scope exists");
                scope.insert(name, value);
                Ok(ExecFlow::Continue)
            }
            Statement::Assign { name, value } => {
                let value = self.eval_expr(value)?;
                // Update the nearest scope that already defines `name`.
                for scope in self.scopes.iter_mut().rev() {
                    if scope.contains_key(name) {
                        scope.insert(*name, value);
                        return Ok(ExecFlow::Continue);
                    }
                }
                Err(RuntimeError::new(format!("unknown variable `{name}`")))
            }
            Statement::If {
                condition,
                then,
                else_if,
                else_block,
            } => {
                if self.eval_expr(condition)?.as_bool()? {
                    return self.exec_block(then);
                }
                for (else_if_condition, else_if_block) in else_if {
                    if self.eval_expr(else_if_condition)?.as_bool()? {
                        return self.exec_block(else_if_block);
                    }
                }
                if let Some(else_block) = else_block {
                    return self.exec_block(else_block);
                }
                Ok(ExecFlow::Continue)
            }
            Statement::While { condition, body } => {
                while self.eval_expr(condition)?.as_bool()? {
                    let flow = self.exec_block(body)?;
                    if matches!(flow, ExecFlow::Return(_)) {
                        return Ok(flow);
                    }
                }
                Ok(ExecFlow::Continue)
            }
            Statement::Return(value) => {
                let value = if let Some(value) = value {
                    self.eval_expr(value)?
                } else {
                    Value::Unit
                };
                Ok(ExecFlow::Return(value))
            }
            Statement::Expr(expr) => {
                self.eval_expr(expr)?;
                Ok(ExecFlow::Continue)
            }
        }
    }

    fn eval_expr(&mut self, expr: &Expr<'src>) -> Result<Value, RuntimeError> {
        match expr {
            Expr::Num(v) => Ok(Value::Num(*v)),
            Expr::Str(v) => Ok(Value::Str((*v).to_owned())),
            Expr::Bool(v) => Ok(Value::Bool(*v)),
            Expr::Var(name) => self.lookup_var(name),
            Expr::Group(inner) => self.eval_expr(inner),
            Expr::UnaryOp { operand, op } => {
                let value = self.eval_expr(operand)?;
                match op {
                    UnaryOp::Neg => Ok(Value::Num(-value.as_num()?)),
                    UnaryOp::Not => Ok(Value::Bool(!value.as_bool()?)),
                }
            }
            Expr::BinOp { lhand, rhand, op } => match op {
                BinOp::And => {
                    let lhs = self.eval_expr(lhand)?;
                    if !lhs.as_bool()? {
                        return Ok(Value::Bool(false));
                    }
                    Ok(Value::Bool(self.eval_expr(rhand)?.as_bool()?))
                }
                BinOp::Or => {
                    let lhs = self.eval_expr(lhand)?;
                    if lhs.as_bool()? {
                        return Ok(Value::Bool(true));
                    }
                    Ok(Value::Bool(self.eval_expr(rhand)?.as_bool()?))
                }
                BinOp::Add => {
                    let lhs = self.eval_expr(lhand)?;
                    let rhs = self.eval_expr(rhand)?;
                    match (lhs, rhs) {
                        (Value::Num(l), Value::Num(r)) => Ok(Value::Num(l + r)),
                        (Value::Str(l), Value::Str(r)) => Ok(Value::Str(l + &r)),
                        _ => Err(RuntimeError::new("operator `+` expects number+number or string+string")),
                    }
                }
                BinOp::Sub => Ok(Value::Num(self.eval_expr(lhand)?.as_num()? - self.eval_expr(rhand)?.as_num()?)),
                BinOp::Mul => Ok(Value::Num(self.eval_expr(lhand)?.as_num()? * self.eval_expr(rhand)?.as_num()?)),
                BinOp::Div => Ok(Value::Num(self.eval_expr(lhand)?.as_num()? / self.eval_expr(rhand)?.as_num()?)),
                BinOp::Less => Ok(Value::Bool(self.eval_expr(lhand)?.as_num()? < self.eval_expr(rhand)?.as_num()?)),
                BinOp::LessOrEqual => Ok(Value::Bool(self.eval_expr(lhand)?.as_num()? <= self.eval_expr(rhand)?.as_num()?)),
                BinOp::Greater => Ok(Value::Bool(self.eval_expr(lhand)?.as_num()? > self.eval_expr(rhand)?.as_num()?)),
                BinOp::GreaterOrEqual => Ok(Value::Bool(self.eval_expr(lhand)?.as_num()? >= self.eval_expr(rhand)?.as_num()?)),
                BinOp::Equal => {
                    let lhs = self.eval_expr(lhand)?;
                    let rhs = self.eval_expr(rhand)?;
                    Ok(Value::Bool(values_equal(&lhs, &rhs)))
                }
            },
            Expr::FuncCall { name, args } => {
                let mut evaluated_args = Vec::with_capacity(args.len());
                for arg in args {
                    evaluated_args.push(self.eval_expr(arg)?);
                }
                self.call_function(name, evaluated_args)
            }
            Expr::Invalid(source) => Err(RuntimeError::new(format!(
                "cannot evaluate invalid expression `{source}`"
            ))),
        }
    }

    fn lookup_var(&self, name: &str) -> Result<Value, RuntimeError> {
        for scope in self.scopes.iter().rev() {
            if let Some(value) = scope.get(name) {
                return Ok(value.clone());
            }
        }
        Err(RuntimeError::new(format!("unknown variable `{name}`")))
    }
}

impl Value {
    fn as_num(&self) -> Result<f64, RuntimeError> {
        match self {
            Value::Num(v) => Ok(*v),
            _ => Err(RuntimeError::new("expected number")),
        }
    }

    fn as_bool(&self) -> Result<bool, RuntimeError> {
        match self {
            Value::Bool(v) => Ok(*v),
            _ => Err(RuntimeError::new("expected bool")),
        }
    }
}

fn values_equal(lhs: &Value, rhs: &Value) -> bool {
    match (lhs, rhs) {
        (Value::Num(l), Value::Num(r)) => l == r,
        (Value::Str(l), Value::Str(r)) => l == r,
        (Value::Bool(l), Value::Bool(r)) => l == r,
        (Value::Unit, Value::Unit) => true,
        _ => false,
    }
}

fn builtin_print(args: &[Value]) -> Result<Value, RuntimeError> {
    for (index, value) in args.iter().enumerate() {
        if index > 0 {
            print!(" ");
        }
        print!("{value}");
    }
    println!();
    Ok(Value::Unit)
}

fn builtin_input(args: &[Value]) -> Result<Value, RuntimeError> {
    if let Some(prompt) = args.first() {
        print!("{prompt}");
        io::stdout()
            .flush()
            .map_err(|err| RuntimeError::new(format!("failed to flush prompt: {err}")))?;
    }
    let mut line = String::new();
    io::stdin()
        .read_line(&mut line)
        .map_err(|err| RuntimeError::new(format!("failed to read input: {err}")))?;
    if line.ends_with('\n') {
        line.pop();
        if line.ends_with('\r') {
            line.pop();
        }
    }
    Ok(Value::Str(line))
}

fn builtin_str_to_num(args: &[Value]) -> Result<Value, RuntimeError> {
    let Some(Value::Str(s)) = args.first() else {
        return Err(RuntimeError::new("builtin `str_to_num` expects a string"));
    };
    let s = s.trim();
    let n = s.parse::<f64>().map_err(|_| {
        RuntimeError::new(format!(
            "builtin `str_to_num` failed to parse `{}` as a number",
            s
        ))
    })?;
    Ok(Value::Num(n))
}

fn builtin_num_to_str(args: &[Value]) -> Result<Value, RuntimeError> {
    let Some(Value::Num(n)) = args.first() else {
        return Err(RuntimeError::new("builtin `num_to_str` expects a number"));
    };
    Ok(Value::Str(n.to_string()))
}

pub fn run_file<'src>(functions: &'src [FunctionDef<'src>]) -> Result<Value, RuntimeError> {
    Runtime::new(functions).run()
}
