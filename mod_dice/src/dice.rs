use rand::seq::SliceRandom;
use rand::thread_rng;
use rand::Rng;
use std::borrow::Cow;
use std::fmt;
use std::fmt::Display;
use std::iter;

use rustbot::prelude::{span_join, Color, Format, Span};
use rustbot::{span, spans};

// enums
use self::EvaluatedValue::*;

// space eater
named!(space<&str,&str>, eat_separator!(" \t"));
macro_rules! sp (
  ($i:expr, $($args:tt)*) => (
    {
      match sep!($i, space, $($args)*) {
        Err(e) => Err(e),
        Ok((i1,o))    => {
          match space(i1) {
            Err(e) => Err(e),
            Ok((i2,_))    => Ok((i2, o))
          }
        }
      }
    }
  )
);

pub struct EvaluationLimiter {
    entropy: u64,
}

impl EvaluationLimiter {
    pub fn new(entropy: u64) -> Self {
        Self { entropy }
    }

    fn use_entropy(&mut self, count: u64, options: u64) -> Result<(), String> {
        let entropy = match options
            .checked_next_power_of_two()
            .map(|v| v.trailing_zeros())
            .map(|v| count.checked_mul(v as u64))
            .flatten()
        {
            Some(v) => v,
            None => return Err("overflow calculating entropy".to_string()),
        };

        println!(
            "trying to use {}x {} options = {} bits, out of {} bits remaining",
            count, options, entropy, self.entropy
        );
        if self.entropy < entropy {
            Err("roll too complex".to_string())
        } else {
            self.entropy -= entropy;
            Ok(())
        }
    }
}

pub fn parse(input: &str) -> Result<Expression, String> {
    fullexpr(&format!("{}\n", input))
        .map(|(_, c)| c)
        .map_err(|e| format!("{:?}", e))
}

pub fn eval(expr: &Expression, mut limit: EvaluationLimiter) -> Result<Vec<Span>, String> {
    let (s, v) = expr.eval(&mut limit)?;
    Ok(spans!(v.to_string(), ": ", s))
}

trait Evaluable {
    fn eval(&self, limit: &mut EvaluationLimiter) -> Result<(Vec<Span>, EvaluatedValue), String>;
}
enum EvaluatedValue {
    Integer(i64),
    IntSlice(Vec<i64>),
    Bool(bool),
    BoolSlice(Vec<bool>),
}
impl Display for EvaluatedValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            Integer(i) => write!(f, "{}", i),
            IntSlice(s) => {
                if s.len() <= 10 {
                    let strs: Vec<String> = s.iter().map(|v| format!("{}", v)).collect();
                    write!(f, "[{}]", strs.join(", "))
                } else {
                    write!(f, "[{} ints, total {}]", s.len(), s.iter().sum::<i64>())
                }
            }
            Bool(b) => write!(f, "{}", b),
            BoolSlice(s) => {
                if s.len() <= 10 {
                    let strs: Vec<String> = s.iter().map(|v| format!("{}", v)).collect();
                    write!(f, "[{}]", strs.join(", "))
                } else {
                    write!(f, "[{} bools, {} true]", s.len(), s.iter().filter(|v| **v).count())
                }
            }
        }
    }
}

impl EvaluatedValue {
    fn as_i64(&self) -> Result<i64, String> {
        match self {
            Integer(i) => Ok(*i),
            IntSlice(s) => Ok(s.iter().sum()),
            Bool(true) => Ok(1),
            Bool(false) => Ok(0),
            BoolSlice(s) => Ok(s.iter().filter(|&v| *v).count() as i64),
        }
    }
    fn as_int_slice(&self) -> Result<Vec<i64>, String> {
        match self {
            Integer(i) => Err(format!("cannot convert {} to slice", i)),
            IntSlice(s) => Ok(s.to_vec()),
            Bool(b) => Err(format!("cannot convert {} to slice", b)),
            BoolSlice(s) => Ok(s.iter().map(|&v| if v { 1 } else { 0 }).collect()),
        }
    }
}

named!(fullexpr<&str, Expression>,
    terminated!(expression, tag!("\n"))
);

named!(expression<&str, Expression>,
    map!(repeat, |v| Expression{expr: v})
);
#[derive(Debug)]
pub struct Expression {
    pub expr: Repeat, // ...
}
impl Evaluable for Expression {
    fn eval(&self, limit: &mut EvaluationLimiter) -> Result<(Vec<Span>, EvaluatedValue), String> {
        self.expr.eval(limit)
    }
}

named!(repeat<&str, Repeat>, sp!(alt!(
    do_parse!(
        n: number >>
        tag!("#") >>
        c: comparison >>
        (Repeat{ repeat: Some(n), term: c })
    ) |
    map!(comparison, |v| Repeat{ repeat: None, term: v })
)));
#[derive(Debug)]
pub struct Repeat {
    pub repeat: Option<i64>, // ( integer "#" )?
    pub term: Comparison,    // ...
}
impl Evaluable for Repeat {
    fn eval(&self, limit: &mut EvaluationLimiter) -> Result<(Vec<Span>, EvaluatedValue), String> {
        match self.repeat {
            None => self.term.eval(limit),
            Some(n) => {
                let (strs, vals) = (0..n)
                    .map(|_| {
                        let (s, v) = self.term.eval(limit)?;
                        Ok((s, v.as_i64()?))
                    })
                    .collect::<Result<Vec<(Vec<Span>, _)>, String>>()?
                    .drain(..)
                    .unzip();

                Ok((span_join(strs, ", "), IntSlice(vals)))
            }
        }
    }
}

named!(comparison<&str, Comparison>, sp!(do_parse!(
    l: addsub >>
    r: opt!(tuple!(compare_op, addsub)) >>
    (Comparison{left: l, right: r})
)));
#[derive(Debug)]
pub struct Comparison {
    pub left: AddSub,                       // ...
    pub right: Option<(CompareOp, AddSub)>, // ( operator ... )?
}
impl Evaluable for Comparison {
    fn eval(&self, limit: &mut EvaluationLimiter) -> Result<(Vec<Span>, EvaluatedValue), String> {
        let l = self.left.eval(limit)?;
        match &self.right {
            None => Ok(l),
            Some((op, term)) => {
                let r = term.eval(limit)?;
                let (os, v) = op.apply(l.1, r.1)?;

                let left = spans!(l.0, format!("{}", op), r.0);
                Ok((if os.is_empty() { left } else { spans!(left, "=", os) }, v))
            }
        }
    }
}

named!(addsub<&str, AddSub>, sp!(do_parse!(
    l: muldiv >>
    r: many0!(tuple!(addsub_op, muldiv)) >>
    (AddSub{left: l, right: r})
)));
#[derive(Debug)]
pub struct AddSub {
    pub left: MulDiv,                   // ...
    pub right: Vec<(AddSubOp, MulDiv)>, // ( operator ... )*
}
impl Evaluable for AddSub {
    fn eval(&self, limit: &mut EvaluationLimiter) -> Result<(Vec<Span>, EvaluatedValue), String> {
        let (s, mut l) = self.left.eval(limit)?;
        let mut ss = s;
        for elem in &self.right {
            let (mut rs, r) = elem.1.eval(limit)?;

            ss.push(format!("{}", elem.0).into());
            ss.append(&mut rs);
            l = elem.0.apply(l, r)?;
        }
        Ok((ss, l))
    }
}

named!(muldiv<&str, MulDiv>, sp!(do_parse!(
    l: sum >>
    r: many0!(tuple!(muldiv_op, sum)) >>
    (MulDiv{left: l, right: r})
)));
#[derive(Debug)]
pub struct MulDiv {
    pub left: Sum,                   // ...
    pub right: Vec<(MulDivOp, Sum)>, // ( operator ... )*
}
impl Evaluable for MulDiv {
    fn eval(&self, limit: &mut EvaluationLimiter) -> Result<(Vec<Span>, EvaluatedValue), String> {
        let (s, mut l) = self.left.eval(limit)?;
        let mut ss = s;
        for elem in &self.right {
            let (mut rs, r) = elem.1.eval(limit)?;

            ss.push(format!("{}", elem.0).into());
            ss.append(&mut rs);
            l = elem.0.apply(l, r)?;
        }
        Ok((ss, l))
    }
}

named!(sum<&str, Sum>, sp!(do_parse!(
    s: alt!(value!(true, tag!("s")) | value!(false)) >>
    t: dicemod >>
    (Sum{is_sum: s, term: t})
)));
#[derive(Debug)]
pub struct Sum {
    pub is_sum: bool,  // ( "s" )?
    pub term: DiceMod, // ...
}
impl Evaluable for Sum {
    fn eval(&self, limit: &mut EvaluationLimiter) -> Result<(Vec<Span>, EvaluatedValue), String> {
        let (s, v) = self.term.eval(limit)?;
        if self.is_sum {
            Ok((spans!("s", s), Integer(v.as_i64()?)))
        } else {
            Ok((s, v))
        }
    }
}

named!(dicemod<&str, DiceMod>, sp!(do_parse!(
    r: diceroll >>
    o: opt!(tuple!(dicemod_op, value)) >>
    (DiceMod{roll: r, op: o})
)));
#[derive(Debug)]
pub struct DiceMod {
    pub roll: DiceRoll,             // ...
    pub op: Option<(ModOp, Value)>, // ( operator ... )?
}
impl Evaluable for DiceMod {
    fn eval(&self, limit: &mut EvaluationLimiter) -> Result<(Vec<Span>, EvaluatedValue), String> {
        match &self.op {
            None => self.roll.eval(limit),
            Some((op, r)) => match self.roll {
                DiceRoll::NoRoll(_) => {
                    let l = self.roll.eval(limit)?;
                    let (rs, rv) = r.eval(limit)?;
                    let (_, v) = op.apply(l.1, rv)?;
                    Ok((spans!(l.0, format!("{}", op), rs), v))
                }
                DiceRoll::Roll { .. } => {
                    let (s, l) = self.roll._eval(limit)?;
                    let (rs, rv) = r.eval(limit)?;
                    let (vs, v) = op.apply(l, rv)?;
                    Ok((spans!(s, format!("{}", op), rs, ":", vs), v))
                }
            },
        }
    }
}

named!(explode<&str, Explode>, sp!(alt!(
    do_parse!(
        tag!("!") >>
        n: number >>
        (Explode::Target(n))
    ) |
    value!(Explode::Default, tag!("!"))
)));
#[derive(Debug)]
pub enum Explode {
    Default,
    Target(i64),
}
impl Display for Explode {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        match self {
            Explode::Default => write!(f, "!"),
            Explode::Target(t) => write!(f, "!{}", t),
        }
    }
}

named!(diceroll<&str, DiceRoll>, sp!(alt!(
    do_parse!(
        c: opt!(value) >>
        tag!("d") >>
        s: opt!(value) >>
        e: opt!(explode) >>
        (DiceRoll::Roll{count: c, sides: s, explode: e})
    ) |
    map!(value, |v| DiceRoll::NoRoll(v))
)));
#[derive(Debug)]
pub enum DiceRoll {
    NoRoll(Value), // ...
    Roll {
        count: Option<Value>,     // ( ... )? "d"
        sides: Option<Value>,     // ( ... )?
        explode: Option<Explode>, // ( "!" ( integer )? )?
    },
}
impl Evaluable for DiceRoll {
    fn eval(&self, limit: &mut EvaluationLimiter) -> Result<(Vec<Span>, EvaluatedValue), String> {
        let (s, r) = self._eval(limit)?;
        match self {
            DiceRoll::NoRoll(_) => Ok((s, r)),
            DiceRoll::Roll { .. } => Ok((spans!(s, ":", r.to_string()), r)),
        }
    }
}

enum DiceOptions {
    Vector(Vec<i64>),
    Range(i64, i64),
}
impl DiceOptions {
    fn roll(&self, rng: &mut rand::rngs::ThreadRng) -> i64 {
        match self {
            Self::Vector(v) => *v.choose(rng).unwrap(),
            Self::Range(lo, hi) => rng.gen_range(lo, hi + 1),
        }
    }
    fn is_empty(&self) -> bool {
        match self {
            Self::Vector(v) => v.is_empty(),
            Self::Range(lo, hi) => hi <= lo,
        }
    }
    fn get_max_value(&self) -> i64 {
        match self {
            Self::Vector(v) => *v.iter().max().unwrap(),
            Self::Range(_lo, hi) => *hi,
        }
    }
    fn get_min_value(&self) -> i64 {
        match self {
            Self::Vector(v) => *v.iter().min().unwrap(),
            Self::Range(lo, _hi) => *lo,
        }
    }
    fn get_options(&self) -> u64 {
        match self {
            Self::Vector(v) => v.len() as u64,
            Self::Range(lo, hi) => (hi - lo + 1) as u64,
        }
    }
}

impl DiceRoll {
    fn _eval(&self, limit: &mut EvaluationLimiter) -> Result<(Vec<Span>, EvaluatedValue), String> {
        match self {
            DiceRoll::NoRoll(v) => v.eval(limit),
            DiceRoll::Roll {
                count: cv,
                sides: sv,
                explode: ex,
            } => {
                let (cs, c) = match cv {
                    Some(v) => {
                        let (vs, vv) = v.eval(limit)?;
                        let count = vv.as_i64()?;
                        (vs, count)
                    }
                    None => (vec![], 1),
                };

                if c < 0 {
                    return Err(format!("tried to roll {} dice", c));
                }

                let (ss, s) = match sv {
                    Some(v) => {
                        let (vs, vv) = v.eval(limit)?;
                        let opts: DiceOptions = match vv {
                            Integer(i) if i >= 1 => DiceOptions::Range(1, i),
                            Integer(0) => return Err("cannot roll a d0".to_string()),
                            Integer(i) => return Err(format!("cannot roll a d({})", i)),
                            IntSlice(s) => DiceOptions::Vector(s),
                            Bool(b) => return Err(format!("cannot roll a d{}", b)),
                            BoolSlice(_) => return Err("cannot roll a d[list of bool]".to_string()),
                        };
                        (vs, opts)
                    }
                    None => (vec![], DiceOptions::Range(1, 6)),
                };

                if s.is_empty() {
                    return Err("tried to roll a die with no options".to_string());
                }

                let mut n = c as usize;
                let target = match ex {
                    None => None,
                    Some(Explode::Default) => Some(s.get_max_value()),
                    Some(Explode::Target(t)) => Some(*t),
                };

                if let Some(target) = target {
                    let min_roll = s.get_min_value();
                    if min_roll >= target {
                        return Err("tried to roll an always-exploding die".to_string());
                    }
                }

                let n_options = s.get_options();
                limit.use_entropy(n as u64, n_options)?;

                let mut rng = thread_rng();
                let mut entropy_err = None;
                let results = iter::repeat_with(|| s.roll(&mut rng))
                    .take_while(|&roll| {
                        if n == 0 {
                            return false;
                        }
                        match target {
                            None => n -= 1,
                            Some(target) => {
                                if roll < target {
                                    n -= 1
                                } else {
                                    let e = limit.use_entropy(1, n_options);
                                    if e.is_err() {
                                        entropy_err = Some(e);
                                        return false;
                                    }
                                }
                            }
                        };
                        true
                    })
                    .collect();

                if let Some(e) = entropy_err {
                    e?;
                }

                let exp_str: Cow<str> = match ex {
                    None => "".into(),
                    Some(exp) => format!("{}", exp).into(),
                };
                Ok((spans!(cs, "d", ss, exp_str), IntSlice(results)))
            }
        }
    }
}

named!(value<&str, Value>, sp!(alt!(
    do_parse!(
        sp!(tag!("-")) >>
        v: number >>
        (Value::Integer(-v))
    ) |
    map!(number, |v| Value::Integer(v)) |
    map!(delimited!(tag!("("), expression, tag!(")")), |v| Value::Sub(Box::new(v))) |
    map!(delimited!(tag!("["), separated_list!(tag!(","), expression), tag!("]")), |v| Value::Slice(v)) |
    value!(Value::Fate, tag!("F")) |
    value!(Value::Hundred, tag!("%"))
)));
#[derive(Debug)]
pub enum Value {
    Integer(i64),           // ...
    Sub(Box<Expression>),   // "(" ... ")"
    Slice(Vec<Expression>), // "[" ... "]"
    Fate,                   // "F"
    Hundred,                // "%"
}
impl Evaluable for Value {
    fn eval(&self, limit: &mut EvaluationLimiter) -> Result<(Vec<Span>, EvaluatedValue), String> {
        match self {
            Value::Integer(i) => Ok((spans!(format!("{}", i)), Integer(*i))),
            Value::Sub(expr) => {
                let (es, ev) = expr.eval(limit)?;
                Ok((spans!("(", es, ")"), ev))
            }
            Value::Slice(s) => {
                let (strs, vals) = s
                    .iter()
                    .map(|e| {
                        let (s, v) = e.eval(limit)?;
                        Ok((s, v.as_i64()?))
                    })
                    .collect::<Result<Vec<(Vec<Span>, _)>, String>>()?
                    .drain(..)
                    .unzip();

                Ok((spans!("[", span_join(strs, ", "), "]"), IntSlice(vals)))
            }
            Value::Fate => Ok((spans!("F"), IntSlice(vec![-1, 0, 1]))),
            Value::Hundred => Ok((spans!("%"), Integer(100))),
        }
    }
}

named!(addsub_op<&str, AddSubOp>, alt!(value!(AddSubOp::Add, tag!("+")) | value!(AddSubOp::Sub, tag!("-"))));
#[derive(Debug)]
pub enum AddSubOp {
    Add, // +
    Sub, // -
}
impl AddSubOp {
    fn apply(&self, left: EvaluatedValue, right: EvaluatedValue) -> Result<EvaluatedValue, String> {
        if let (IntSlice(l), IntSlice(r)) = (&left, &right) {
            let mut l = l.clone();
            l.extend_from_slice(&r);
            return Ok(IntSlice(l));
        }
        let l = left.as_i64()?;
        let r = right.as_i64()?;
        let result = match self {
            AddSubOp::Add => l.wrapping_add(r),
            AddSubOp::Sub => l.wrapping_sub(r),
        };
        Ok(Integer(result))
    }
}
impl Display for AddSubOp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AddSubOp::Add => write!(f, "+"),
            AddSubOp::Sub => write!(f, "-"),
        }
    }
}

named!(muldiv_op<&str, MulDivOp>, alt!(value!(MulDivOp::Mul, tag!("*")) | value!(MulDivOp::Div, tag!("/"))));
#[derive(Debug)]
pub enum MulDivOp {
    Mul, // *
    Div, // /
}
impl MulDivOp {
    fn apply(&self, left: EvaluatedValue, right: EvaluatedValue) -> Result<EvaluatedValue, String> {
        let l = left.as_i64()?;
        let r = right.as_i64()?;
        let result = match self {
            MulDivOp::Mul => l.wrapping_mul(r),
            MulDivOp::Div => l.wrapping_div(r),
        };
        Ok(Integer(result))
    }
}
impl Display for MulDivOp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            MulDivOp::Mul => write!(f, "*"),
            MulDivOp::Div => write!(f, "/"),
        }
    }
}

named!(compare_op<&str, CompareOp>, sp!(alt!(
    value!(CompareOp::LessEq, alt!(tag!("<=") | tag!("=<"))) |
    value!(CompareOp::Less, tag!("<")) |
    value!(CompareOp::GreaterEq, alt!(tag!(">=") | tag!("=>"))) |
    value!(CompareOp::Greater, tag!(">")) |
    value!(CompareOp::Equal, alt!(tag!("==") | tag!("="))) |
    value!(CompareOp::Unequal, alt!(tag!("!=") | tag!("<>")))
)));
#[derive(Debug)]
pub enum CompareOp {
    Less,      // <
    LessEq,    // <=, =<
    Greater,   // >
    GreaterEq, // >=, =>
    Equal,     // ==, =
    Unequal,   // !=, <>
}
impl CompareOp {
    fn compare(&self, l: i64, r: i64) -> bool {
        match self {
            CompareOp::Less => l < r,
            CompareOp::LessEq => l <= r,
            CompareOp::Greater => l > r,
            CompareOp::GreaterEq => l >= r,
            CompareOp::Equal => l == r,
            CompareOp::Unequal => l != r,
        }
    }
    fn apply(&self, left: EvaluatedValue, right: EvaluatedValue) -> Result<(Vec<Span>, EvaluatedValue), String> {
        let l = match left {
            Integer(v) => Ok(v),
            IntSlice(v) => IntSlice(v).as_i64(),
            v => Err(format!("cannot compare {} {} {}", v, self, right)),
        }?;
        match right {
            Integer(r) => Ok((vec![], Bool(self.compare(l, r)))),
            IntSlice(s) => {
                let (strings, values): (Vec<Span>, Vec<bool>) = s
                    .iter()
                    .map(|r| {
                        if self.compare(l, *r) {
                            (span!(Color::Green; "{}", *r), true)
                        } else {
                            (span!(Color::Red; "{}", *r), false)
                        }
                    })
                    .unzip();
                Ok((spans!("[", span_join(strings, ", "), "]"), BoolSlice(values)))
            }
            v => Err(format!("cannot compare {} {} {}", l, self, v)),
        }
    }
}
impl Display for CompareOp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CompareOp::Less => write!(f, "<"),
            CompareOp::LessEq => write!(f, "<="),
            CompareOp::Greater => write!(f, ">"),
            CompareOp::GreaterEq => write!(f, ">="),
            CompareOp::Equal => write!(f, "=="),
            CompareOp::Unequal => write!(f, "!="),
        }
    }
}

named!(dicemod_op<&str, ModOp>, alt!(
    value!(ModOp::DropLowest, tag!("l")) |
    value!(ModOp::DropHighest, tag!("h")) |
    value!(ModOp::KeepLowest, tag!("L")) |
    value!(ModOp::KeepHighest, tag!("H"))
));
#[derive(Debug)]
pub enum ModOp {
    DropLowest,  // l
    DropHighest, // h
    KeepLowest,  // L
    KeepHighest, // H
}

fn format_arrays(ac: Color, aa: &[i64], bc: Color, ba: &[i64]) -> Vec<Span<'static>> {
    let vec = Iterator::chain(
        aa.iter().map(|v| span!(ac + Format::Bold; "{}", v)),
        ba.iter().map(|v| span!(bc + Format::Bold; "{}", v)),
    )
    .collect::<Vec<_>>();
    spans!("[", span_join(vec, ", "), "]")
}

impl ModOp {
    fn apply(&self, left: EvaluatedValue, right: EvaluatedValue) -> Result<(Vec<Span>, EvaluatedValue), String> {
        let mut l = left.as_int_slice()?;
        l.sort();
        let r = right.as_i64()? as usize;
        if r > l.len() {
            return Err(format!(
                "cannot evaluate a keep/drop {} operation on {} dice",
                r,
                l.len()
            ));
        }
        let (s, result) = match self {
            ModOp::DropLowest => (format_arrays(Color::Red, &l[..r], Color::Yellow, &l[r..]), &l[r..]),
            ModOp::DropHighest => {
                let i = l.len() - r;
                (format_arrays(Color::Yellow, &l[..i], Color::Red, &l[i..]), &l[..i])
            }
            ModOp::KeepLowest => (format_arrays(Color::Yellow, &l[..r], Color::Red, &l[r..]), &l[..r]),
            ModOp::KeepHighest => {
                let i = l.len() - r;
                (format_arrays(Color::Red, &l[..i], Color::Yellow, &l[i..]), &l[i..])
            }
        };
        Ok((s, IntSlice(result.to_vec())))
    }
}
impl Display for ModOp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ModOp::DropLowest => write!(f, "l"),
            ModOp::DropHighest => write!(f, "h"),
            ModOp::KeepLowest => write!(f, "L"),
            ModOp::KeepHighest => write!(f, "H"),
        }
    }
}

named!(number<&str, i64>,
    map_res!(take_while!(is_digit), |s: &str| s.parse::<i64>())
);

fn is_digit(c: char) -> bool {
    '0' <= c && c <= '9'
}
