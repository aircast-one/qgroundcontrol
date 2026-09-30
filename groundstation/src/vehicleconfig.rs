use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::control::decode;
use crate::read::{flag, object};
use crate::router::Backend;

const APM_FLIGHT_SAFETY: &str = include_str!("../../src/AutoPilotPlugins/APM/VehicleConfig/APMFlightSafety.VehicleConfig.json");
const APM_FAILSAFES: &str = include_str!("../../src/AutoPilotPlugins/APM/VehicleConfig/APMFailsafes.VehicleConfig.json");
const APM_LOGGING: &str = include_str!("../../src/AutoPilotPlugins/APM/VehicleConfig/APMLogging.VehicleConfig.json");
const APM_POWER: &str = include_str!("../../src/AutoPilotPlugins/APM/VehicleConfig/APMPower.VehicleConfig.json");
const APM_TUNING_COPTER: &str = include_str!("../../src/AutoPilotPlugins/APM/VehicleConfig/APMTuningCopter.VehicleConfig.json");
const PX4_SAFETY: &str = include_str!("../../src/AutoPilotPlugins/PX4/VehicleConfig/Safety.VehicleConfig.json");
const PX4_POWER: &str = include_str!("../../src/AutoPilotPlugins/PX4/VehicleConfig/Power.VehicleConfig.json");

const CONFIGS: &[(&str, bool, &str)] = &[
    ("Flight Safety", false, APM_FLIGHT_SAFETY),
    ("Failsafes", false, APM_FAILSAFES),
    ("Logging", false, APM_LOGGING),
    ("Power", false, APM_POWER),
    ("Tuning", false, APM_TUNING_COPTER),
    ("Safety", true, PX4_SAFETY),
    ("Power", true, PX4_POWER),
];

const ROW_PREFIX: &str = "vehicleConfig(";
const ENUM_INDEX: &str = ".enumIndex";
const VALIDATE: &str = ".validate";

pub fn has(page: &str, px4: bool) -> bool {
    CONFIGS.iter().any(|(name, firmware, _)| *name == page && *firmware == px4)
}

fn config(page: &str, px4: bool) -> Option<Value> {
    CONFIGS.iter().find(|(name, firmware, _)| *name == page && *firmware == px4).and_then(|(_, _, text)| serde_json::from_str(text).ok())
}

fn parameter_path(name: &str) -> String {
    format!("vehicle.parameterManager.getParameter(-1,{name})")
}

#[derive(Clone, Debug, PartialEq)]
enum Val {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Fact(String),
    List(Vec<Val>),
}

impl Val {
    fn truthy(&self) -> bool {
        match self {
            Val::Null => false,
            Val::Bool(b) => *b,
            Val::Num(n) => *n != 0.0 && !n.is_nan(),
            Val::Str(s) => !s.is_empty(),
            Val::Fact(_) | Val::List(_) => true,
        }
    }

    fn number(&self) -> Option<f64> {
        match self {
            Val::Num(n) => Some(*n),
            Val::Bool(b) => Some(f64::from(u8::from(*b))),
            _ => None,
        }
    }
}

fn from_json(value: &Value) -> Val {
    match value {
        Value::Bool(b) => Val::Bool(*b),
        Value::Number(n) => n.as_f64().map_or(Val::Null, Val::Num),
        Value::String(s) => Val::Str(s.clone()),
        _ => Val::Null,
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Num(f64),
    Str(String),
    Ident(String),
    Op(&'static str),
}

const OPERATORS: &[&str] = &["===", "!==", "==", "!=", ">=", "<=", "&&", "||", "!", "&", "<", ">", "?", ":", "(", ")", "[", "]", ",", ".", "+", "-", "*", "/"];

fn tokenize(text: &str) -> Option<Vec<Tok>> {
    let chars: Vec<char> = text.chars().collect();
    let scan = |start: usize| -> Option<(usize, Tok)> {
        let c = chars[start];
        let run = |keep: &dyn Fn(char) -> bool| (start..chars.len()).find(|i| !keep(chars[*i])).unwrap_or(chars.len());
        match c {
            '"' => (start + 1..chars.len()).find(|i| chars[*i] == '"').map(|end| (end + 1, Tok::Str(chars[start + 1..end].iter().collect()))),
            _ if c.is_ascii_digit() => {
                let end = run(&|c| c.is_ascii_digit() || c == '.');
                chars[start..end].iter().collect::<String>().parse().ok().map(|n| (end, Tok::Num(n)))
            }
            _ if c.is_alphabetic() || c == '_' => {
                let end = run(&|c| c.is_alphanumeric() || c == '_');
                Some((end, Tok::Ident(chars[start..end].iter().collect())))
            }
            _ => {
                let rest: String = chars[start..].iter().collect();
                OPERATORS.iter().find(|op| rest.starts_with(**op)).map(|op| (start + op.len(), Tok::Op(op)))
            }
        }
    };
    std::iter::successors(Some((0usize, None::<Option<Tok>>)), |(at, previous)| {
        if matches!(previous, Some(None)) {
            return None;
        }
        let start = (*at..chars.len()).find(|i| !chars[*i].is_whitespace())?;
        Some(scan(start).map_or((chars.len(), Some(None)), |(end, tok)| (end, Some(Some(tok)))))
    })
    .skip(1)
    .map(|(_, tok)| tok.flatten())
    .collect()
}

#[derive(Clone, Debug)]
enum Expr {
    Lit(Val),
    Name(String),
    Member(Box<Expr>, String),
    Call(Box<Expr>, Vec<Expr>),
    List(Vec<Expr>),
    Not(Box<Expr>),
    Neg(Box<Expr>),
    Binary(&'static str, Box<Expr>, Box<Expr>),
    Ternary(Box<Expr>, Box<Expr>, Box<Expr>),
}

fn binding_power(op: &str) -> Option<u8> {
    match op {
        "?" => Some(1),
        "||" => Some(2),
        "&&" => Some(3),
        "&" => Some(4),
        "===" | "!==" | "==" | "!=" => Some(5),
        "<" | ">" | "<=" | ">=" => Some(6),
        "+" | "-" => Some(7),
        "*" | "/" => Some(8),
        _ => None,
    }
}

struct Parser {
    toks: Vec<Tok>,
    at: std::cell::Cell<usize>,
}

impl Parser {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.at.get())
    }

    fn next(&self) -> Option<Tok> {
        let tok = self.toks.get(self.at.get()).cloned();
        self.at.set(self.at.get() + 1);
        tok
    }

    fn eat(&self, op: &str) -> bool {
        let matched = matches!(self.peek(), Some(Tok::Op(o)) if *o == op);
        if matched {
            self.at.set(self.at.get() + 1);
        }
        matched
    }

    fn list(&self, close: &str) -> Option<Vec<Expr>> {
        match self.eat(close) {
            true => Some(vec![]),
            false => {
                let items: Vec<Expr> = std::iter::successors(Some(self.expr(0)?), |_| self.eat(",").then(|| self.expr(0)).flatten()).collect();
                self.eat(close).then_some(items)
            }
        }
    }

    fn primary(&self) -> Option<Expr> {
        let base = match self.next()? {
            Tok::Num(n) => Expr::Lit(Val::Num(n)),
            Tok::Str(s) => Expr::Lit(Val::Str(s)),
            Tok::Ident(name) if name == "true" => Expr::Lit(Val::Bool(true)),
            Tok::Ident(name) if name == "false" => Expr::Lit(Val::Bool(false)),
            Tok::Ident(name) => Expr::Name(name),
            Tok::Op("!") => return Some(Expr::Not(Box::new(self.primary()?))),
            Tok::Op("-") => return Some(Expr::Neg(Box::new(self.primary()?))),
            Tok::Op("(") => {
                let inner = self.expr(0)?;
                self.eat(")").then_some(inner)?
            }
            Tok::Op("[") => Expr::List(self.list("]")?),
            _ => return None,
        };
        Some(self.postfix(base))
    }

    fn postfix(&self, base: Expr) -> Expr {
        if self.eat(".") {
            return match self.next() {
                Some(Tok::Ident(member)) => self.postfix(Expr::Member(Box::new(base), member)),
                _ => base,
            };
        }
        match self.eat("(").then(|| self.list(")")).flatten() {
            Some(args) => self.postfix(Expr::Call(Box::new(base), args)),
            None => base,
        }
    }

    fn expr(&self, min: u8) -> Option<Expr> {
        let lhs = self.primary()?;
        self.climb(lhs, min)
    }

    fn climb(&self, lhs: Expr, min: u8) -> Option<Expr> {
        let Some(Tok::Op(op)) = self.peek().cloned() else { return Some(lhs) };
        let Some(power) = binding_power(op).filter(|p| *p > min) else { return Some(lhs) };
        self.next();
        let combined = match op {
            "?" => {
                let yes = self.expr(0)?;
                self.eat(":").then_some(())?;
                let no = self.expr(0)?;
                Expr::Ternary(Box::new(lhs), Box::new(yes), Box::new(no))
            }
            _ => Expr::Binary(op, Box::new(lhs), Box::new(self.expr(power)?)),
        };
        self.climb(combined, min)
    }
}

fn parse(text: &str) -> Option<Expr> {
    let parser = Parser { toks: tokenize(text)?, at: std::cell::Cell::new(0) };
    let parsed = parser.expr(0)?;
    (parser.at.get() == parser.toks.len()).then_some(parsed)
}

#[derive(Clone)]
struct Repeat {
    prefix: String,
}

struct Scope<'a> {
    backend: &'a dyn Backend,
    config: &'a Value,
    multi_rotor: bool,
    fixed_wing: bool,
    repeat: Option<Repeat>,
    locals: BTreeMap<String, Val>,
}

impl<'a> Scope<'a> {
    fn with(&self, repeat: Option<Repeat>, locals: BTreeMap<String, Val>) -> Scope<'a> {
        Scope { backend: self.backend, config: self.config, multi_rotor: self.multi_rotor, fixed_wing: self.fixed_wing, repeat, locals }
    }

    fn fact(&self, name: &str) -> Option<Value> {
        let fact = object(&self.backend.get(&parameter_path(name)));
        let present = fact.get("kind").and_then(Value::as_str) == Some("fact") && fact.get("name").and_then(Value::as_str).is_some_and(|n| !n.is_empty());
        present.then_some(fact)
    }

    fn exists(&self, name: &str) -> bool {
        self.fact(name).is_some()
    }

    fn full_name(&self, postfix: &str) -> String {
        self.repeat.as_ref().map_or_else(|| postfix.to_string(), |r| format!("{}{postfix}", r.prefix))
    }

    fn name(&self, name: &str) -> Val {
        if let Some(local) = self.locals.get(name) {
            return local.clone();
        }
        if let Some(constant) = self.config["constants"].get(name) {
            return from_json(constant);
        }
        if let Some(param) = self.config["params"].get(name) {
            let (param_name, exists_only) = match param {
                Value::String(s) => (s.clone(), false),
                other => (other["name"].as_str().unwrap_or_default().to_string(), flag(other, "existsOnly")),
            };
            return match (exists_only, self.exists(&param_name)) {
                (true, exists) => Val::Bool(exists),
                (false, true) => Val::Fact(param_name),
                (false, false) => Val::Null,
            };
        }
        match self.config["bindings"].get(name).and_then(Value::as_str) {
            Some(binding) => self.eval_text(binding),
            None => Val::Null,
        }
    }

    fn fact_member(&self, name: &str, member: &str) -> Val {
        let Some(fact) = self.fact(name) else { return Val::Null };
        match member {
            "rawValue" => from_json(fact.get("rawValue").unwrap_or(&fact["value"])),
            "value" => from_json(&fact["value"]),
            _ => Val::Null,
        }
    }

    fn call(&self, callee: &Expr, args: &[Val]) -> Val {
        let text = |i: usize| match args.get(i) {
            Some(Val::Str(s)) => Some(s.clone()),
            _ => None,
        };
        match callee {
            Expr::Name(f) if f == "_fullParamName" => text(0).map_or(Val::Null, |p| Val::Str(self.full_name(&p))),
            Expr::Member(base, method) if matches!(base.as_ref(), Expr::Name(n) if n == "controller") => match method.as_str() {
                "parameterExists" => Val::Bool(text(1).is_some_and(|n| self.exists(&n))),
                "getParameterFact" => text(1).filter(|n| self.exists(n)).map_or(Val::Null, Val::Fact),
                _ => Val::Null,
            },
            Expr::Member(base, method) if method == "includes" => match (self.eval(base), args.first()) {
                (Val::List(items), Some(wanted)) => Val::Bool(items.iter().any(|item| strict_equal(item, wanted))),
                _ => Val::Bool(false),
            },
            _ => Val::Null,
        }
    }

    fn eval(&self, expr: &Expr) -> Val {
        match expr {
            Expr::Lit(v) => v.clone(),
            Expr::Name(n) => self.name(n),
            Expr::List(items) => Val::List(items.iter().map(|i| self.eval(i)).collect()),
            Expr::Not(inner) => Val::Bool(!self.eval(inner).truthy()),
            Expr::Neg(inner) => self.eval(inner).number().map_or(Val::Null, |n| Val::Num(-n)),
            Expr::Member(base, member) => match (base.as_ref(), member.as_str()) {
                (Expr::Member(root, vehicle), property) if vehicle == "vehicle" && matches!(root.as_ref(), Expr::Name(n) if n == "controller") => match property {
                    "multiRotor" => Val::Bool(self.multi_rotor),
                    "fixedWing" => Val::Bool(self.fixed_wing),
                    _ => Val::Null,
                },
                _ => match self.eval(base) {
                    Val::Fact(name) => self.fact_member(&name, member),
                    _ => Val::Null,
                },
            },
            Expr::Call(callee, args) => {
                let values: Vec<Val> = args.iter().map(|a| self.eval(a)).collect();
                self.call(callee, &values)
            }
            Expr::Ternary(test, yes, no) => match self.eval(test).truthy() {
                true => self.eval(yes),
                false => self.eval(no),
            },
            Expr::Binary(op, lhs, rhs) => self.binary(op, lhs, rhs),
        }
    }

    fn binary(&self, op: &str, lhs: &Expr, rhs: &Expr) -> Val {
        let left = self.eval(lhs);
        match op {
            "&&" => match left.truthy() {
                true => self.eval(rhs),
                false => left,
            },
            "||" => match left.truthy() {
                true => left,
                false => self.eval(rhs),
            },
            _ => {
                let right = self.eval(rhs);
                let numbers = left.number().zip(right.number());
                match op {
                    "===" | "==" => Val::Bool(strict_equal(&left, &right)),
                    "!==" | "!=" => Val::Bool(!strict_equal(&left, &right)),
                    "<" => Val::Bool(numbers.is_some_and(|(a, b)| a < b)),
                    ">" => Val::Bool(numbers.is_some_and(|(a, b)| a > b)),
                    "<=" => Val::Bool(numbers.is_some_and(|(a, b)| a <= b)),
                    ">=" => Val::Bool(numbers.is_some_and(|(a, b)| a >= b)),
                    "&" => numbers.map_or(Val::Num(0.0), |(a, b)| Val::Num(((a as i64) & (b as i64)) as f64)),
                    "+" => numbers.map_or(Val::Null, |(a, b)| Val::Num(a + b)),
                    "-" => numbers.map_or(Val::Null, |(a, b)| Val::Num(a - b)),
                    "*" => numbers.map_or(Val::Null, |(a, b)| Val::Num(a * b)),
                    "/" => numbers.map_or(Val::Null, |(a, b)| Val::Num(a / b)),
                    _ => Val::Null,
                }
            }
        }
    }

    fn eval_text(&self, text: &str) -> Val {
        parse(text).map_or(Val::Null, |e| self.eval(&e))
    }

    fn shown(&self, owner: &Value, key: &str) -> bool {
        owner[key].as_str().is_none_or(|text| self.eval_text(text).truthy())
    }

    fn assign(&self, statement: &str) -> Result<(), String> {
        let body = statement.trim().trim_start_matches('{').trim_end_matches('}').trim();
        let at = body
            .char_indices()
            .find(|(i, c)| *c == '=' && !matches!(body[..*i].chars().last(), Some('=' | '!' | '<' | '>')) && !body[i + 1..].starts_with('='))
            .map(|(i, _)| i)
            .ok_or_else(|| format!("{statement} is not an assignment"))?;
        let target = parse(body[..at].trim()).ok_or_else(|| format!("{statement} has no target"))?;
        let Expr::Member(fact, property) = target else { return Err(format!("{statement} does not write a parameter")) };
        let Val::Fact(name) = self.eval(&fact) else { return Err("That parameter is not on this vehicle.".to_string()) };
        let value = self.eval_text(body[at + 1..].trim()).number().ok_or_else(|| format!("{statement} does not compute a number"))?;
        write_parameter(self.backend, &name, property == "rawValue", value)
    }
}

fn strict_equal(a: &Val, b: &Val) -> bool {
    match (a, b) {
        (Val::Num(x), Val::Num(y)) => x == y,
        _ => a == b,
    }
}

fn write_parameter(backend: &dyn Backend, name: &str, raw: bool, value: f64) -> Result<(), String> {
    let path = parameter_path(name);
    let payload = json!({ "value": value }).to_string();
    let answer = match raw {
        true => object(&backend.set(&format!("{path}.rawValue"), &payload)),
        false => crate::factwrite::write(backend, &path, &payload),
    };
    match flag(&answer, "ok") {
        true => Ok(()),
        false => Err(answer["reason"].as_str().map_or_else(|| format!("{name} did not take {value}."), str::to_string)),
    }
}

fn vehicle_shape(backend: &dyn Backend) -> (bool, bool) {
    let vehicle = object(&backend.get_fields("vehicle", "multiRotor,fixedWing"));
    (flag(&vehicle, "multiRotor"), flag(&vehicle, "fixedWing"))
}

struct Instance {
    repeat: Option<Repeat>,
    index: usize,
    heading: String,
}

fn battery_prefix(prefix: &str, index: usize) -> String {
    match index {
        0 => format!("{prefix}_"),
        1..=8 => format!("{prefix}{}_", index + 1),
        _ => format!("{prefix}{}_", char::from(b'A' + (index - 9) as u8)),
    }
}

fn battery_label(index: usize) -> String {
    match index {
        0..=8 => (index + 1).to_string(),
        _ => char::from(b'A' + (index - 9) as u8).to_string(),
    }
}

fn repeat_prefixes(scope: &Scope, repeat: &Value) -> Vec<(String, String)> {
    let param_prefix = repeat["paramPrefix"].as_str().unwrap_or_default();
    let probe = repeat["probePostfix"].as_str().unwrap_or_default();
    let apm_battery = repeat["indexing"].as_str() == Some("apm_battery");
    let start = repeat["startIndex"].as_u64().unwrap_or(1) as usize;
    let omits_first = flag(repeat, "firstIndexOmitsNumber");
    (0..16)
        .map(|i| match apm_battery {
            true => (battery_prefix(param_prefix, i), battery_label(i)),
            false => {
                let raw = i + start;
                let index = if omits_first && raw == start { String::new() } else { raw.to_string() };
                (format!("{param_prefix}{index}"), raw.to_string())
            }
        })
        .take_while(|(prefix, _)| scope.exists(&format!("{prefix}{probe}")))
        .collect()
}

fn instances(scope: &Scope, section: &Value) -> Vec<Instance> {
    let title = section["title"].as_str().unwrap_or_default();
    match section.get("repeat") {
        None => vec![Instance { repeat: None, index: 0, heading: title.to_string() }],
        Some(repeat) => {
            let prefixes = repeat_prefixes(scope, repeat);
            let many = prefixes.len() > 1;
            prefixes
                .into_iter()
                .enumerate()
                .map(|(index, (prefix, label))| Instance {
                    repeat: Some(Repeat { prefix }),
                    index,
                    heading: match (title.contains("{index}"), many) {
                        (true, _) => title.replace("{index}", &label),
                        (false, true) => format!("{title} {label}"),
                        (false, false) => title.to_string(),
                    },
                })
                .collect()
        }
    }
}

fn instance_enabled(scope: &Scope, repeat: &Value) -> bool {
    match repeat["enableParam"].as_str() {
        None => true,
        Some(enable) => {
            let disabled = scope.eval_text(repeat["disabledParamValue"].as_str().unwrap_or("0"));
            let current = scope.fact_member(&scope.full_name(enable), "value");
            !strict_equal(&current, &disabled)
        }
    }
}

fn row_path(page: &str, id: &str) -> String {
    format!("{ROW_PREFIX}{page}).{id}")
}

fn labelled(mut decoded: Value, control: &Value, enabled: bool) -> Value {
    decoded["label"] = control["label"].clone();
    if let Some(description) = control["description"].as_str() {
        decoded["description"] = json!(description);
    }
    if !enabled {
        decoded["enabled"] = json!(false);
        decoded["disabledReason"] = json!("Not in use with the choice above");
    }
    decoded
}

fn rows(scope: &Scope, page: &str, id: &str, control: &Value) -> Vec<Value> {
    if !scope.shown(control, "showWhen") {
        return vec![];
    }
    let enabled = scope.shown(control, "enableWhen");
    let kind = control["control"].as_str().unwrap_or("");
    let label = control["label"].clone();
    let path = row_path(page, id);
    if kind == "label" {
        return vec![json!({ "control": "label", "label": label, "warning": flag(control, "warning"), "path": path })];
    }
    let (fact_path, fact) = match (control["param"].as_str(), control["setting"].as_str()) {
        (Some(param), _) => {
            let name = scope.full_name(param);
            (parameter_path(&name), scope.fact(&name))
        }
        (None, Some(setting)) => {
            let setting_path = format!("settings.{setting}");
            let read = object(&scope.backend.get(&setting_path));
            (setting_path, (read.get("kind").and_then(Value::as_str) == Some("fact")).then_some(read))
        }
        (None, None) => (String::new(), None),
    };
    match (kind, fact) {
        ("toggleCheckbox", _) => {
            let checked = scope.eval_text(control["toggleCheckbox"]["checked"].as_str().unwrap_or("false")).truthy();
            vec![json!({ "control": "toggle", "label": label, "value": checked, "enabled": enabled, "path": path })]
        }
        (_, None) => vec![],
        ("bitmaskCheckbox", Some(fact)) => {
            let raw = fact.get("rawValue").or(fact.get("value")).and_then(Value::as_f64).unwrap_or(0.0) as i64;
            let bit = control["bitMask"].as_i64().unwrap_or(0);
            vec![json!({ "control": "toggle", "label": label, "value": raw & bit != 0, "enabled": enabled, "path": path })]
        }
        ("radiogroup", Some(_)) => {
            let options: Vec<&Value> = control["options"].as_array().map(|o| o.iter().collect()).unwrap_or_default();
            let chosen = options.iter().find(|o| scope.eval_text(o["checked"].as_str().unwrap_or("false")).truthy());
            vec![json!({
                "control": "choice",
                "label": label,
                "options": options.iter().enumerate().map(|(i, o)| json!({ "label": o["label"], "raw": i.to_string() })).collect::<Vec<_>>(),
                "display": chosen.map_or(Value::Null, |o| o["label"].clone()),
                "enabled": enabled,
                "path": path,
            })]
        }
        ("factslider", Some(fact)) if control.get("linkedParams").is_some() => vec![labelled(decode(&fact, &path), control, enabled)],
        (_, Some(fact)) => {
            let row = labelled(decode(&fact, &fact_path), control, enabled);
            match control.get("enableCheckbox") {
                None => vec![row],
                Some(toggle) => {
                    let checked = scope.eval_text(toggle["checked"].as_str().unwrap_or("false")).truthy();
                    let gated = match checked {
                        true => row,
                        false => labelled(row, control, false),
                    };
                    vec![json!({ "control": "toggle", "label": label, "value": checked, "enabled": enabled, "path": format!("{path}.enable") }), gated]
                }
            }
        }
    }
}

fn scope_for<'a>(backend: &'a dyn Backend, config: &'a Value) -> Scope<'a> {
    let (multi_rotor, fixed_wing) = vehicle_shape(backend);
    Scope { backend, config, multi_rotor, fixed_wing, repeat: None, locals: BTreeMap::new() }
}

fn disabled_companion(scope: &Scope, page: &str, section_index: usize, section: &Value) -> Option<Value> {
    let repeat = section.get("repeat")?;
    let companion = repeat.get("disabledSection")?;
    let enable = repeat["enableParam"].as_str()?;
    let disabled = scope.eval_text(repeat["disabledParamValue"].as_str().unwrap_or("0"));
    let prefixes = repeat_prefixes(scope, repeat);
    let many = prefixes.len() > 1;
    let title = section["title"].as_str().unwrap_or_default();
    let controls: Vec<Value> = prefixes
        .iter()
        .enumerate()
        .filter_map(|(index, (prefix, label))| {
            let name = format!("{prefix}{enable}");
            let fact = scope.fact(&name)?;
            strict_equal(&from_json(&fact["value"]), &disabled).then(|| {
                let heading = if many { format!("{title} {label}") } else { title.to_string() };
                match companion.get("enabledParamValue") {
                    Some(_) => json!({ "control": "toggle", "label": heading, "value": false, "path": row_path(page, &format!("{section_index}.{index}.disabled")) }),
                    None => labelled(decode(&fact, &parameter_path(&name)), &json!({ "label": heading }), true),
                }
            })
        })
        .collect();
    (!controls.is_empty()).then(|| json!({ "title": companion["heading"], "note": "", "controls": controls }))
}

pub fn page(backend: &dyn Backend, page: &str, px4: bool) -> Value {
    let Some(config) = config(page, px4) else { return crate::read::refused(&format!("{page} has no VehicleConfig definition for this firmware")) };
    let base = scope_for(backend, &config);
    let sections: Vec<Value> = config["sections"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .enumerate()
        .flat_map(|(section_index, section)| {
            let shown: Vec<Value> = match base.shown(section, "showWhen") {
                false => vec![],
                true => instances(&base, section)
                    .into_iter()
                    .map(|instance| {
                        let scope = base.with(instance.repeat, BTreeMap::new());
                        let enabled = section.get("repeat").is_none_or(|r| instance_enabled(&scope, r));
                        let controls: Vec<Value> = match enabled {
                            false => vec![],
                            true => section["controls"]
                                .as_array()
                                .cloned()
                                .unwrap_or_default()
                                .iter()
                                .enumerate()
                                .flat_map(|(control_index, control)| rows(&scope, page, &format!("{section_index}.{}.{control_index}", instance.index), control))
                                .collect(),
                        };
                        json!({ "title": instance.heading, "note": "", "controls": controls })
                    })
                    .collect(),
            };
            shown.into_iter().chain(disabled_companion(&base, page, section_index, section))
        })
        .filter(|s| s["controls"].as_array().is_some_and(|c| c.iter().any(|row| row["control"] != "label")))
        .collect();
    json!({ "kind": "object", "class": "SetupPage", "page": page, "firmware": if px4 { "px4" } else { "apm" }, "available": !sections.is_empty(), "sections": sections })
}

struct Target {
    page: String,
    section: usize,
    instance: usize,
    control: Option<usize>,
    suffix: String,
}

fn target(path: &str) -> Option<Target> {
    let rest = path.strip_prefix(ROW_PREFIX)?;
    let (page, id) = rest.split_once(").")?;
    let parts: Vec<&str> = id.split('.').collect();
    let section = parts.first()?.parse().ok()?;
    let instance = parts.get(1)?.parse().ok()?;
    let (control, suffix) = match parts.get(2).copied()? {
        "disabled" => (None, "disabled".to_string()),
        index => (Some(index.parse().ok()?), parts.get(3).map_or(String::new(), |s| (*s).to_string())),
    };
    if parts.len() > 4 || !matches!(suffix.as_str(), "" | "disabled" | "enable" | "enumIndex") {
        return None;
    }
    Some(Target { page: page.to_string(), section, instance, control, suffix })
}

pub fn owns(path: &str) -> bool {
    target(path).is_some()
}

pub fn owns_validate(path: &str) -> bool {
    path.strip_suffix(VALIDATE).is_some_and(owns)
}

fn px4(backend: &dyn Backend) -> bool {
    flag(&object(&backend.get_fields("vehicle", "px4Firmware")), "px4Firmware")
}

pub fn validate(backend: &dyn Backend, path: &str, args: &str) -> Value {
    let row = path.strip_suffix(VALIDATE).unwrap_or(path);
    match resolve(backend, row) {
        Ok(resolved) => {
            let scope = scope_for(backend, &resolved.config).with(resolved.repeat.clone(), BTreeMap::new());
            let name = scope.full_name(resolved.control["param"].as_str().unwrap_or_default());
            crate::factwrite::validate(backend, &format!("{}{VALIDATE}", parameter_path(&name)), args)
        }
        Err(reason) => json!({ "ok": false, "result": reason }),
    }
}

struct Resolved {
    config: Value,
    repeat: Option<Repeat>,
    control: Value,
    at: Target,
}

fn resolve(backend: &dyn Backend, path: &str) -> Result<Resolved, String> {
    let at = target(path.strip_suffix(ENUM_INDEX).unwrap_or(path)).ok_or_else(|| format!("{path} is not a setup row"))?;
    let config = config(&at.page, px4(backend)).ok_or_else(|| format!("{} has no definition for this firmware", at.page))?;
    let section = config["sections"].get(at.section).cloned().ok_or("That section is gone.")?;
    let repeat = match section.get("repeat") {
        None => None,
        Some(_) => instances(&scope_for(backend, &config), &section).into_iter().nth(at.instance).ok_or("That battery is no longer reported.")?.repeat,
    };
    let control = at.control.and_then(|i| section["controls"].get(i).cloned()).unwrap_or(section);
    Ok(Resolved { config, repeat, control, at })
}

fn answer(result: Result<(), String>) -> Value {
    match result {
        Ok(()) => json!({ "ok": true, "result": true }),
        Err(reason) => json!({ "ok": false, "result": false, "reason": reason }),
    }
}

pub fn write(backend: &dyn Backend, path: &str, value: &str) -> Value {
    let asked = serde_json::from_str::<Value>(value).ok().and_then(|v| v.get("value").cloned()).unwrap_or(Value::Null);
    let resolved = match resolve(backend, path) {
        Ok(resolved) => resolved,
        Err(reason) => return answer(Err(reason)),
    };
    let base = scope_for(backend, &resolved.config);
    let scope = base.with(resolved.repeat.clone(), BTreeMap::new());
    let (control, at) = (&resolved.control, &resolved.at);
    let checked = asked.as_bool().or_else(|| asked.as_f64().map(|n| n != 0.0)).unwrap_or(false);
    let number = asked.as_f64().or_else(|| asked.as_str().and_then(|s| s.trim().parse().ok()));
    let kind = control["control"].as_str().unwrap_or("");
    answer(match (at.suffix.as_str(), kind, path.ends_with(ENUM_INDEX)) {
        ("disabled", ..) => {
            let repeat = &control["repeat"];
            let name = repeat_prefixes(&scope, repeat).get(at.instance).map(|(prefix, _)| format!("{prefix}{}", repeat["enableParam"].as_str().unwrap_or_default()));
            let chosen = scope.eval_text(repeat["disabledSection"]["enabledParamValue"].as_str().unwrap_or("0")).number();
            match (name, chosen, checked) {
                (Some(name), Some(enabled), true) => write_parameter(backend, &name, false, enabled),
                (Some(_), _, false) => Ok(()),
                _ => Err("That battery is no longer reported.".to_string()),
            }
        }
        ("enable", ..) => {
            let locals = BTreeMap::from([("enableCheckBoxChecked".to_string(), Val::Bool(checked))]);
            scope.with(resolved.repeat.clone(), locals).assign(control["enableCheckbox"]["onClicked"].as_str().unwrap_or_default())
        }
        (_, "toggleCheckbox", _) => scope.assign(control["toggleCheckbox"][if checked { "onChecked" } else { "onUnchecked" }].as_str().unwrap_or_default()),
        (_, "bitmaskCheckbox", _) => {
            let name = scope.full_name(control["param"].as_str().unwrap_or_default());
            let raw = scope.fact_member(&name, "rawValue").number().unwrap_or(0.0) as i64;
            let bit = control["bitMask"].as_i64().unwrap_or(0);
            write_parameter(backend, &name, true, (if checked { raw | bit } else { raw & !bit }) as f64)
        }
        (_, "radiogroup", true) => {
            let name = scope.full_name(control["param"].as_str().unwrap_or_default());
            let option = asked.as_u64().or_else(|| asked.as_str().and_then(|s| s.parse().ok())).and_then(|i| control["options"].get(i as usize));
            match option.and_then(|o| scope.eval_text(o["value"].as_str().unwrap_or_default()).number()) {
                Some(v) => write_parameter(backend, &name, flag(control, "raw"), v),
                None => Err("Choose one of the listed options.".to_string()),
            }
        }
        (_, "factslider", _) => {
            let name = scope.full_name(control["param"].as_str().unwrap_or_default());
            match number {
                None => Err("This setting is a number.".to_string()),
                Some(v) => write_parameter(backend, &name, false, v).and_then(|()| {
                    let locals = BTreeMap::from([("value".to_string(), Val::Num(v))]);
                    let linked = scope.with(resolved.repeat.clone(), locals);
                    control["linkedParams"].as_object().cloned().unwrap_or_default().iter().try_for_each(|(other, expression)| {
                        let computed = linked.eval_text(expression.as_str().unwrap_or("value")).number().ok_or_else(|| format!("{other} could not be computed"))?;
                        write_parameter(backend, other, false, computed)
                    })
                }),
            }
        }
        _ => Err(format!("{path} is not a row that writes")),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    struct Fake {
        params: RefCell<BTreeMap<String, f64>>,
        multi_rotor: bool,
        px4: bool,
        writes: RefCell<Vec<(String, f64)>>,
    }

    impl Fake {
        fn new(params: &[(&str, f64)]) -> Self {
            Fake { params: RefCell::new(params.iter().map(|(n, v)| (n.to_string(), *v)).collect()), multi_rotor: true, px4: false, writes: RefCell::new(vec![]) }
        }
    }

    fn name_of(path: &str) -> Option<&str> {
        path.strip_prefix("vehicle.parameterManager.getParameter(-1,").and_then(|p| p.split(')').next())
    }

    impl Backend for Fake {
        fn get(&self, path: &str) -> String {
            match name_of(path).and_then(|n| self.params.borrow().get(n).map(|v| (n.to_string(), *v))) {
                Some((name, value)) => json!({ "kind": "fact", "name": name, "value": value, "rawValue": value, "valueString": value.to_string() }),
                None => json!({ "kind": "null" }),
            }
            .to_string()
        }
        fn get_fields(&self, _p: &str, _f: &str) -> String {
            json!({ "kind": "object", "multiRotor": self.multi_rotor, "fixedWing": !self.multi_rotor, "px4Firmware": self.px4 }).to_string()
        }
        fn set(&self, path: &str, value: &str) -> String {
            let v = serde_json::from_str::<Value>(value).unwrap()["value"].as_f64().unwrap();
            let name = name_of(path).unwrap().to_string();
            self.params.borrow_mut().insert(name.clone(), v);
            self.writes.borrow_mut().push((name, v));
            json!({ "ok": true }).to_string()
        }
        fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
        fn watch(&self, _p: &[String]) {}
    }

    fn eval(fake: &Fake, config: &Value, text: &str) -> Val {
        scope_for(fake, config).eval_text(text)
    }

    #[test]
    fn expressions_follow_javascript() {
        let fake = Fake::new(&[("FENCE_TYPE", 5.0), ("FS_OPTIONS", 0.0)]);
        let config = json!({ "constants": { "_one": 1 }, "params": { "_fenceType": "FENCE_TYPE", "_missing": "NOPE", "_opts": { "name": "FS_OPTIONS", "existsOnly": true } }, "bindings": { "_fenced": "_fenceType && _fenceType.rawValue !== 0" } });
        assert_eq!(eval(&fake, &config, "_fenced"), Val::Bool(true));
        assert_eq!(eval(&fake, &config, "_fenceType && (_fenceType.rawValue & 4)"), Val::Num(4.0), "&& yields its right operand, and & binds tighter than &&");
        assert_eq!(eval(&fake, &config, "_missing && _missing.rawValue === 0"), Val::Null, "a missing parameter short-circuits rather than reading as zero");
        assert_eq!(eval(&fake, &config, "_opts"), Val::Bool(true));
        assert_eq!(eval(&fake, &config, "controller.vehicle.multiRotor && !controller.vehicle.fixedWing"), Val::Bool(true));
        assert_eq!(eval(&fake, &config, "[3, 4, 5].includes(controller.getParameterFact(-1, \"FENCE_TYPE\").rawValue)"), Val::Bool(true));
        assert_eq!(eval(&fake, &config, "controller.parameterExists(-1, \"NOPE\")"), Val::Bool(false));
        assert_eq!(eval(&fake, &config, "_one > 0 ? 15 : 1500"), Val::Num(15.0));
        assert_eq!(eval(&fake, &config, "-1 < _one"), Val::Bool(true));
    }

    #[test]
    fn every_shipped_expression_parses() {
        CONFIGS.iter().for_each(|(page, _, text)| {
            let config: Value = serde_json::from_str(text).unwrap();
            let strings = |v: &Value, keys: &[&str]| -> Vec<String> { keys.iter().filter_map(|k| v[*k].as_str().map(str::to_string)).collect() };
            let from_sections: Vec<String> = config["sections"].as_array().unwrap().iter().flat_map(|s| {
                strings(s, &["showWhen"]).into_iter().chain(s["controls"].as_array().cloned().unwrap_or_default().iter().flat_map(|c| {
                    strings(c, &["showWhen", "enableWhen"])
                        .into_iter()
                        .chain(strings(&c["toggleCheckbox"], &["checked"]))
                        .chain(strings(&c["enableCheckbox"], &["checked"]))
                        .chain(c["options"].as_array().cloned().unwrap_or_default().iter().flat_map(|o| strings(o, &["value", "checked"])))
                        .collect::<Vec<_>>()
                }).collect::<Vec<_>>())
            }).collect();
            let bindings: Vec<String> = config["bindings"].as_object().map(|b| b.values().filter_map(|v| v.as_str().map(str::to_string)).collect()).unwrap_or_default();
            from_sections.iter().chain(bindings.iter()).for_each(|text| assert!(parse(text).is_some(), "{page}: {text}"));
        });
    }

    #[test]
    fn failsafes_show_copter_rows_and_hide_the_disabled_detail() {
        let fake = Fake::new(&[("FS_GCS_ENABLE", 0.0), ("FS_GCS_TIMEOUT", 5.0), ("FS_THR_ENABLE", 1.0), ("FS_THR_VALUE", 975.0), ("FS_OPTIONS", 0.0), ("FS_CRASH_CHECK", 1.0), ("BATT_MONITOR", 4.0), ("BATT_FS_LOW_ACT", 2.0), ("BATT2_MONITOR", 0.0), ("BATT2_FS_LOW_ACT", 0.0)]);
        let served = page(&fake, "Failsafes", false);
        let titles: Vec<&str> = served["sections"].as_array().unwrap().iter().map(|s| s["title"].as_str().unwrap()).collect();
        assert!(titles.contains(&"Battery Failsafe 1"), "{titles:?}");
        assert!(!titles.contains(&"Battery Failsafe 2"), "a battery whose monitor is off shows no failsafe section, as QGC's repeater hides it");
        let gcs = served["sections"].as_array().unwrap().iter().find(|s| s["title"] == "Ground Station Failsafe").unwrap();
        assert_eq!(gcs["controls"].as_array().unwrap().len(), 1, "with FS_GCS_ENABLE off only the Enabled toggle shows");
        assert_eq!(gcs["controls"][0]["value"], false);
        let throttle = served["sections"].as_array().unwrap().iter().find(|s| s["title"] == "Throttle Failsafe").unwrap();
        assert_eq!(throttle["controls"][2]["display"], "Always RTL");
    }

    #[test]
    fn toggles_and_radios_write_what_the_definition_says() {
        let fake = Fake::new(&[("FS_GCS_ENABLE", 0.0), ("FS_GCS_TIMEOUT", 5.0), ("FS_OPTIONS", 0.0), ("FENCE_ENABLE", 1.0), ("FENCE_TYPE", 0.0)]);
        let served = page(&fake, "Failsafes", false);
        let gcs = served["sections"].as_array().unwrap().iter().find(|s| s["title"] == "Ground Station Failsafe").unwrap();
        let toggle = gcs["controls"][0]["path"].as_str().unwrap().to_string();
        assert!(owns(&toggle));
        assert_eq!(write(&fake, &toggle, r#"{"value":true}"#)["ok"], true);
        assert_eq!(fake.params.borrow()["FS_GCS_ENABLE"], 1.0, "onChecked sets the RTL action");
        let refreshed = page(&fake, "Failsafes", false);
        let radio = refreshed["sections"].as_array().unwrap().iter().find(|s| s["title"] == "Ground Station Failsafe").unwrap()["controls"][2].clone();
        assert_eq!(radio["control"], "choice");
        assert_eq!(write(&fake, &format!("{}{ENUM_INDEX}", radio["path"].as_str().unwrap()), r#"{"value":1}"#)["ok"], true);
        assert_eq!(fake.params.borrow()["FS_GCS_ENABLE"], 5.0, "the second option is Land");

        let fence = page(&fake, "Flight Safety", false);
        let circle = fence["sections"].as_array().unwrap().iter().find(|s| s["title"] == "GeoFence").unwrap()["controls"].as_array().unwrap().iter().find(|c| c["label"] == "Circle centered on Home").unwrap().clone();
        assert_eq!(write(&fake, circle["path"].as_str().unwrap(), r#"{"value":true}"#)["ok"], true);
        assert_eq!(fake.params.borrow()["FENCE_TYPE"], 2.0, "a bitmask checkbox sets only its own bit");
    }

    #[test]
    fn only_the_suffixes_a_row_serves_are_owned() {
        assert!(owns("vehicleConfig(Failsafes).1.0.0") && owns("vehicleConfig(Failsafes).1.0.2.enumIndex") && owns("vehicleConfig(Power).0.1.disabled"));
        assert!(!owns("vehicleConfig(Failsafes).1.0.0.validate"), "a validate is an invoke, and treating it as a write would flip the toggle it names");
        assert!(owns_validate("vehicleConfig(Tuning).0.0.1.validate"));
        assert!(!owns("vehicleConfig(Failsafes).1.0.0.nonsense") && !owns("vehicleConfig(Failsafes).x.0.0"));
    }

    #[test]
    fn a_linked_slider_writes_its_partners() {
        let fake = Fake::new(&[("PSC_D_ACC_P", 0.5), ("PSC_D_ACC_I", 1.0)]);
        let served = page(&fake, "Tuning", false);
        let climb = served["sections"][0]["controls"].as_array().unwrap().iter().find(|c| c["label"] == "Climb Sensitivity").unwrap().clone();
        assert_eq!(write(&fake, climb["path"].as_str().unwrap(), r#"{"value":0.25}"#)["ok"], true);
        assert_eq!(fake.params.borrow()["PSC_D_ACC_I"], 0.5, "QGC keeps I at twice P");
    }

    #[test]
    fn disabled_batteries_are_offered_to_switch_on() {
        let fake = Fake::new(&[("BATT_MONITOR", 4.0), ("BATT_CAPACITY", 3300.0), ("BATT2_MONITOR", 0.0), ("BATT2_CAPACITY", 0.0)]);
        let served = page(&fake, "Power", false);
        let disabled = served["sections"].as_array().unwrap().iter().find(|s| s["title"] == "Disabled Batteries").unwrap().clone();
        assert_eq!(disabled["controls"][0]["label"], "Battery 2");
        assert_eq!(write(&fake, disabled["controls"][0]["path"].as_str().unwrap(), r#"{"value":true}"#)["ok"], true);
        assert_eq!(fake.params.borrow()["BATT2_MONITOR"], 4.0, "switching one on picks analog voltage and current, as QGC's checkbox does");
    }

    #[test]
    fn an_enable_checkbox_gates_its_slider() {
        let mut fake = Fake::new(&[("GF_MAX_HOR_DIST", 0.0), ("GF_ACTION", 1.0), ("GF_MAX_VER_DIST", 50.0), ("CP_DIST", -1.0), ("RTL_LAND_DELAY", 0.0), ("COM_DISARM_LAND", 2.0)]);
        fake.px4 = true;
        let served = page(&fake, "Safety", true);
        let fence = served["sections"].as_array().unwrap().iter().find(|s| s["title"] == "Geofence Failsafe").unwrap().clone();
        let enable = fence["controls"][1].clone();
        assert_eq!((enable["control"].as_str(), enable["value"].as_bool()), (Some("toggle"), Some(false)));
        assert_eq!(fence["controls"][2]["enabled"], false, "the radius is inert until its checkbox is on");
        assert_eq!(write(&fake, enable["path"].as_str().unwrap(), r#"{"value":true}"#)["ok"], true);
        assert_eq!(fake.params.borrow()["GF_MAX_HOR_DIST"], 100.0);
    }
}
