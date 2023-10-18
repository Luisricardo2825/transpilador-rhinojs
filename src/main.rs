use pest::iterators::Pairs;
use pest::pratt_parser::PrattParser;
use pest::Parser;
use std::{collections::HashMap, io};

#[derive(pest_derive::Parser)]
#[grammar = "grammar.pest"]
pub struct LangParser;

lazy_static::lazy_static! {
    static ref PRATT_PARSER: PrattParser<Rule> = {
        use pest::pratt_parser::{Assoc::*, Op};
        use Rule::*;

        // Precedence is defined lowest to highest
        PrattParser::new()
            // Addition and subtract have equal precedence
            .op(Op::infix(add, Left) | Op::infix(subtract, Left))
            .op(Op::infix(multiply, Left) | Op::infix(divide, Left) | Op::infix(modulo, Left))
            .op(Op::prefix(unary_minus))
    };
}

#[derive(PartialEq, Debug, Clone)]
pub enum Op {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum Verb {
    Plus,
    Increment,
    Decrement,
    And,
    Or,
    Not,
    Lt,
    Lte,
    Gt,
    Gte,
    Eq,
    Neq,
}

#[derive(PartialEq, Debug, Clone)]
pub enum Expr {
    Terms(Vec<Expr>),
    IsGlobal {
        modifier: String,
        ident: String,
        expr: Box<Expr>,
    },
    Number(f64),
    String(String),
    UnaryMinus(Box<Expr>),
    BinOp {
        lhs: Box<Expr>,
        op: Op,
        rhs: Box<Expr>,
    },
    Primitives(Primitives),
    MonadicOp {
        verb: Verb,
        expr: Box<Expr>,
    },
    DyadicOp {
        verb: Verb,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
}

pub fn parse_expr(pairs: Pairs<Rule>, var_pool: &mut Vec<Variable>) -> Expr {
    if pairs.len() > 0 {
        PRATT_PARSER
            .map_primary(|primary| resolve_rule(primary, var_pool))
            .map_infix(|lhs, op, rhs| {
                let op = match op.as_rule() {
                    Rule::add => Op::Add,
                    Rule::subtract => Op::Subtract,
                    Rule::multiply => Op::Multiply,
                    Rule::divide => Op::Divide,
                    Rule::modulo => Op::Modulo,
                    rule => unreachable!("Expr::parse expected infix operation, found {:?}", rule),
                };
                Expr::BinOp {
                    lhs: Box::new(lhs),
                    op,
                    rhs: Box::new(rhs),
                }
            })
            .map_prefix(|op, rhs| match op.as_rule() {
                Rule::unary_minus => Expr::UnaryMinus(Box::new(rhs)),
                _ => unreachable!(),
            })
            .parse(pairs)
    } else {
        panic!("Empty expression: {:?}", pairs);
    }
}

fn resolve_rule(primary: pest::iterators::Pair<'_, Rule>, var_pool: &mut Vec<Variable>) -> Expr {
    match primary.as_rule() {
        Rule::expr => parse_expr(primary.into_inner(), var_pool),
        Rule::number => Expr::Number(primary.as_str().trim().parse::<f64>().unwrap()),
        Rule::mathExpr => parse_expr(primary.into_inner(), var_pool),
        Rule::string => {
            let str = primary.as_str().to_owned();

            let str = &str[1..str.len() - 1];

            let str = str.replace("\\\"", "\"");
            Expr::String(String::from(&str[..]))
        }
        Rule::array => {
            let mut arr = vec![];
            for ele in primary.into_inner() {
                let ast = parse_expr(ele.into_inner(), var_pool);
                arr.push(eval(ast));
            }
            Expr::Primitives(Primitives::Array(arr))
        }
        Rule::object => {
            let mut obj = HashMap::new();
            for ele in primary.into_inner() {
                let mut pair = ele.into_inner();
                let key = pair.next().unwrap();
                let value = pair.next().unwrap_or(key.clone());
                let key = key.as_str();
                let value = resolve_rule(value, var_pool);
                obj.insert(String::from(key), eval(value));
            }
            Expr::Primitives(Primitives::Object(obj))
        }
        Rule::value => parse_expr(primary.into_inner(), var_pool),
        Rule::ident => {
            // get value from var_pool
            let ident = primary.as_str();
            let ident = String::from(ident);
            for ele in &mut *var_pool {
                if ele.ident == ident {
                    return ele.value.to_owned();
                }
            }
            Expr::Primitives(Primitives::Null)
        }
        Rule::assgmtExpr => {
            let mut pair = primary.into_inner();
            let modifier = pair.next().unwrap();
            let ident = pair.next().unwrap();

            let expr = pair.next().unwrap();

            let expr = resolve_rule(expr, var_pool);

            let modifier = modifier.as_str();
            let ident = ident.as_str();
            let mut found = false;
            // Check if var exists
            for ele in var_pool.clone() {
                if ele.ident == ident.to_string() {
                    found = true;
                    if ele.is_const {
                        panic!("Cannot modify const variable");
                    }
                    break;
                }
            }
            if found {
                // Remove var
                var_pool.retain(|ele| ele.ident != ident.to_string());
            }
            let value = Expr::IsGlobal {
                modifier: String::from(modifier),
                ident: String::from(ident),
                expr: Box::new(expr.clone()),
            };
            // Insert var
            var_pool.push(Variable {
                value: value,
                is_const: if modifier == "const" { true } else { false },
                ident: ident.to_string(),
            });
            Expr::IsGlobal {
                modifier: String::from(modifier),
                ident: String::from(ident),
                expr: Box::new(expr),
            }
        }
        Rule::monadicExpr => {
            let mut pair = primary.into_inner();
            let verb = pair.next().unwrap();
            let expr = pair.next().unwrap();
            parse_monadic_verb(verb, resolve_rule(expr, var_pool))
        }
        Rule::dyadicExpr => {
            let mut pairs = primary.into_inner();
            let lhs = pairs.next().unwrap();
            let verb = pairs.next().unwrap();
            let rhs = pairs.next().unwrap();
            let lhs = resolve_rule(lhs, var_pool);

            let rhs = resolve_rule(rhs, var_pool);

            parse_dyadic_verb(verb, lhs, rhs)
        }
        Rule::terms => {
            let terms: Vec<Expr> = primary
                .into_inner()
                .map(|node| resolve_rule(node, var_pool))
                .collect();
            // If there's just a single term, return it without
            // wrapping it in a Terms node.
            match terms.len() {
                1 => terms.get(0).unwrap().clone(),
                _ => Expr::Terms(terms),
            }
        }
        rule => unreachable!("Expr::parse expected atom, found {:?}", rule),
    }
}

fn parse_dyadic_verb(pair: pest::iterators::Pair<Rule>, lhs: Expr, rhs: Expr) -> Expr {
    Expr::DyadicOp {
        lhs: Box::new(lhs),
        rhs: Box::new(rhs),
        verb: match pair.as_str() {
            "+" => Verb::Plus,
            "!" => Verb::Not,
            "!=" => Verb::Neq,
            "==" => Verb::Eq,
            _ => panic!("Unexpected dyadic verb: {}", pair.as_str()),
        },
    }
}

fn parse_monadic_verb(pair: pest::iterators::Pair<Rule>, expr: Expr) -> Expr {
    Expr::MonadicOp {
        verb: match pair.as_str() {
            "++" => Verb::Increment,
            "!" => Verb::Not,
            _ => panic!("Unsupported monadic verb: {}", pair.as_str()),
        },
        expr: Box::new(expr),
    }
}

fn main() -> io::Result<()> {
    let mut var_pool: Vec<Variable> = vec![];
    let unparsed_file = std::fs::read_to_string("example.er").expect("cannot read jsc file");
    let pairs = LangParser::parse(Rule::program, &unparsed_file).expect("Erro parsing");
    for pair in pairs {
        let str = pair.as_str();
        if str.len() <= 0 {
            continue;
        }

        // println!("{str}");
        let exprs = parse_expr(pair.into_inner(), &mut var_pool);
        // println!("{:?}", exprs);
        let result = eval(exprs);

        println!("{:?}", result.to_string());
    }

    Ok(())
}

#[derive(PartialEq, Debug, Clone)]

pub struct Variable {
    pub ident: String,
    pub value: Expr,
    pub is_const: bool,
}

#[derive(PartialEq, Debug, Clone)]
pub enum Primitives {
    String(String),
    Boolean(bool),
    Number(f64),
    Array(Vec<Primitives>),
    Object(HashMap<String, Primitives>),
    Null,
}

fn eval(expr: Expr) -> Primitives {
    match expr {
        Expr::Number(i) => Primitives::Number(i),
        Expr::UnaryMinus(e) => {
            let result = -eval(*e).to_number();
            Primitives::Number(result)
        }
        Expr::BinOp { lhs, op, rhs } => {
            let lhs = eval(*lhs).to_number();
            let rhs = eval(*rhs).to_number();
            let result = match op {
                Op::Add => lhs + rhs,
                Op::Subtract => lhs - rhs,
                Op::Multiply => lhs * rhs,
                Op::Divide => lhs / rhs,
                Op::Modulo => lhs % rhs,
            };
            Primitives::Number(result)
        }
        Expr::String(value) => Primitives::String(value),
        Expr::Primitives(value) => value,
        Expr::IsGlobal {
            modifier: _,
            ident: _,
            expr,
        } => {
            let value = eval(*expr);

            value
        }
        Expr::MonadicOp { verb: _, expr: _ } => todo!(),
        Expr::DyadicOp { verb, lhs, rhs } => match verb {
            Verb::Plus => {
                let lhs = eval(*lhs);
                let rhs = eval(*rhs);
                if lhs.is_string() || rhs.is_string() {
                    return Primitives::String(lhs.concat_string(&rhs));
                }
                Primitives::Number(lhs.to_number() + rhs.to_number())
            }
            Verb::Increment => todo!(),
            Verb::Decrement => todo!(),
            Verb::And => todo!(),
            Verb::Or => todo!(),
            Verb::Not => todo!(),
            Verb::Lt => todo!(),
            Verb::Lte => todo!(),
            Verb::Gt => todo!(),
            Verb::Gte => todo!(),
            Verb::Eq => todo!(),
            Verb::Neq => todo!(),
        },
        Expr::Terms(_) => todo!(),
    }
}

impl Primitives {
    pub fn to_string(&self) -> String {
        match self {
            Primitives::String(value) => value.to_owned(),
            Primitives::Number(value) => value.to_string(),
            Primitives::Boolean(value) => value.to_string(),
            Primitives::Array(value) => serialize_jsonvalue(&Primitives::Array(value.to_owned())),
            Primitives::Object(value) => serialize_jsonvalue(&Primitives::Object(value.to_owned())),
            Primitives::Null => "".to_owned(),
        }
    }

    pub fn to_value(&self) -> String {
        match self {
            Primitives::String(value) => value.to_owned(),
            Primitives::Number(value) => value.to_string(),
            Primitives::Boolean(value) => value.to_string(),
            Primitives::Array(value) => serialize_jsonvalue(&Primitives::Array(value.to_owned())),
            Primitives::Object(value) => serialize_jsonvalue(&Primitives::Object(value.to_owned())),
            Primitives::Null => "null".to_owned(),
        }
    }

    pub fn concat_string(&self, rhs: &Primitives) -> String {
        let lhs = self.to_string();
        let rhs = rhs.to_string();
        lhs + &rhs
    }

    pub fn to_integer(&self) -> i64 {
        match self {
            Primitives::Number(value) => value.round() as i64,
            unknow => panic!("Expected integer, got: {:?}", unknow),
        }
    }
    pub fn to_number(&self) -> f64 {
        match self {
            Primitives::Number(value) => *value,
            Primitives::Null => 0.0,
            un => panic!("Expected float, got: {:?}", un),
        }
    }
    pub fn to_boolean(&self) -> bool {
        match self {
            Primitives::Boolean(value) => *value,
            _ => panic!("Expected boolean"),
        }
    }
    pub fn to_array(&self) -> Vec<Primitives> {
        match self {
            Primitives::Array(value) => value.to_owned(),
            _ => panic!("Expected array"),
        }
    }
    pub fn to_object(&self) -> HashMap<String, Primitives> {
        match self {
            Primitives::Object(value) => value.to_owned(),
            _ => panic!("Expected object"),
        }
    }

    pub fn is_null(&self) -> bool {
        match self {
            Primitives::Null => true,
            _ => false,
        }
    }
    pub fn is_string(&self) -> bool {
        match self {
            Primitives::String(_) => true,
            _ => false,
        }
    }
    pub fn is_boolean(&self) -> bool {
        match self {
            Primitives::Boolean(_) => true,
            _ => false,
        }
    }
    pub fn is_array(&self) -> bool {
        match self {
            Primitives::Array(_) => true,
            _ => false,
        }
    }
    pub fn is_number(&self) -> bool {
        match self {
            Primitives::Number(_) => true,
            _ => false,
        }
    }
}

fn serialize_jsonvalue(val: &Primitives) -> String {
    use Primitives::*;

    match val {
        Object(o) => {
            let contents: Vec<_> = o
                .iter()
                .map(|(name, value)| format!("\"{}\":{}", name, serialize_jsonvalue(value)))
                .collect();
            format!("{{{}}}", contents.join(","))
        }
        Array(a) => {
            let contents: Vec<_> = a.iter().map(serialize_jsonvalue).collect();
            format!("[{}]", contents.join(","))
        }
        String(s) => format!("\"{}\"", s),
        Number(n) => format!("{}", n),
        Boolean(b) => format!("{}", b),
        Null => format!("null"),
    }
}