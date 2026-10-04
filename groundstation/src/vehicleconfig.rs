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

const APM_GIMBAL: &str = include_str!("vehicleconfig/APMGimbal.VehicleConfig.json");
const APM_AIRSPEED: &str = include_str!("vehicleconfig/APMAirspeed.VehicleConfig.json");
const APM_ESC: &str = include_str!("vehicleconfig/APMESC.VehicleConfig.json");
const APM_FLIGHT_MODE: &str = include_str!("vehicleconfig/APMFlightMode.VehicleConfig.json");
const PX4_FLIGHT_MODE: &str = include_str!("vehicleconfig/PX4FlightMode.VehicleConfig.json");
const APM_SIMPLE_MODES: &str = include_str!("vehicleconfig/APMSimpleModes.VehicleConfig.json");
pub const SIMPLE_MODES: &str = "Simple Modes";
const APM_BATTERY_INDICATOR: &str = include_str!("vehicleconfig/APMBatteryIndicator.VehicleConfig.json");
const PX4_BATTERY_INDICATOR: &str = include_str!("vehicleconfig/PX4BatteryIndicator.VehicleConfig.json");
pub const BATTERY_SETTINGS: &str = "Battery Settings";
const APM_MAIN_STATUS: &str = include_str!("vehicleconfig/APMMainStatusIndicator.VehicleConfig.json");
const PX4_MAIN_STATUS: &str = include_str!("vehicleconfig/PX4MainStatusIndicator.VehicleConfig.json");
pub const STATUS_SETTINGS: &str = "Status Settings";
const APM_LIGHTS: &str = include_str!("vehicleconfig/APMLights.VehicleConfig.json");
const PX4_FLIGHT_BEHAVIOR: &str = include_str!("vehicleconfig/PX4FlightBehavior.VehicleConfig.json");
const APM_FLIGHT_SAFETY_SUB: &str = include_str!("vehicleconfig/APMFlightSafetySub.VehicleConfig.json");
const FLIGHT_SAFETY: &str = "Flight Safety";
const FLIGHT_SAFETY_SUB: &str = "Flight Safety (Sub)";
const PX4_RADIO_SWITCHES: &str = include_str!("vehicleconfig/PX4RadioSwitches.VehicleConfig.json");
pub const RADIO_SWITCHES: &str = "Radio Switches";
pub const FLIGHT_MODE_SETTINGS: &str = "Flight Mode Settings";

const CONFIGS: &[(&str, bool, &str)] = &[
    ("Gimbal", false, APM_GIMBAL),
    ("Airspeed", false, APM_AIRSPEED),
    ("ESC", false, APM_ESC),
    (FLIGHT_SAFETY, false, APM_FLIGHT_SAFETY),
    (FLIGHT_SAFETY_SUB, false, APM_FLIGHT_SAFETY_SUB),
    ("Failsafes", false, APM_FAILSAFES),
    ("Logging", false, APM_LOGGING),
    ("Power", false, APM_POWER),
    ("Tuning", false, APM_TUNING_COPTER),
    ("Safety", true, PX4_SAFETY),
    ("Power", true, PX4_POWER),
    (FLIGHT_MODE_SETTINGS, false, APM_FLIGHT_MODE),
    (FLIGHT_MODE_SETTINGS, true, PX4_FLIGHT_MODE),
    (SIMPLE_MODES, false, APM_SIMPLE_MODES),
    (BATTERY_SETTINGS, false, APM_BATTERY_INDICATOR),
    (BATTERY_SETTINGS, true, PX4_BATTERY_INDICATOR),
    (STATUS_SETTINGS, false, APM_MAIN_STATUS),
    (STATUS_SETTINGS, true, PX4_MAIN_STATUS),
    ("Lights", false, APM_LIGHTS),
    ("Flight Behavior", true, PX4_FLIGHT_BEHAVIOR),
    (RADIO_SWITCHES, true, PX4_RADIO_SWITCHES),
];

const ROW_PREFIX: &str = "vehicleConfig(";
const ENUM_INDEX: &str = ".enumIndex";
const VALIDATE: &str = ".validate";

pub fn has(page: &str, px4: bool) -> bool {
    CONFIGS.iter().any(|(name, firmware, _)| *name == page && *firmware == px4)
}

fn config(backend: &dyn Backend, page: &str, px4: bool) -> Option<Value> {
    let sub = !px4 && page == FLIGHT_SAFETY && flag(&object(&backend.get_fields("vehicle", "sub")), "sub");
    let page = if sub { FLIGHT_SAFETY_SUB } else { page };
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
        Value::Array(items) => Val::List(items.iter().map(from_json).collect()),
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
        if let Some(initial) = self.config["state"].get(name) {
            return Val::Bool(page_state().get(&state_key(self.backend, name)).copied().unwrap_or_else(|| initial.as_bool().unwrap_or(false)));
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
                "getParameterFact" => match text(1).filter(|n| self.exists(n)) {
                    Some(name) => Val::Fact(name),
                    None => {
                        text(1).filter(|_| args.get(2) != Some(&Val::Bool(false))).into_iter().for_each(|name| UNMET.with_borrow_mut(|unmet| unmet.iter_mut().for_each(|names| names.push(name.clone()))));
                        Val::Null
                    }
                },
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
        statement.trim().trim_start_matches('{').trim_end_matches('}').split(';').map(str::trim).filter(|one| !one.is_empty()).try_for_each(|one| self.assign_one(one))
    }

    fn assign_one(&self, statement: &str) -> Result<(), String> {
        let body = statement;
        let at = body
            .char_indices()
            .find(|(i, c)| *c == '=' && !matches!(body[..*i].chars().last(), Some('=' | '!' | '<' | '>')) && !body[i + 1..].starts_with('='))
            .map(|(i, _)| i)
            .ok_or_else(|| format!("{statement} is not an assignment"))?;
        let target = parse(body[..at].trim()).ok_or_else(|| format!("{statement} has no target"))?;
        if let Expr::Name(state) = &target {
            if self.config["state"].get(state).is_some() {
                let value = self.eval_text(body[at + 1..].trim()).truthy();
                page_state().insert(state_key(self.backend, state), value);
                return Ok(());
            }
        }
        let Expr::Member(fact, property) = target else { return Err(format!("{statement} does not write a parameter")) };
        let Val::Fact(name) = self.eval(&fact) else { return Err("That parameter is not on this vehicle.".to_string()) };
        let value = self.eval_text(body[at + 1..].trim()).number().ok_or_else(|| format!("{statement} does not compute a number"))?;
        set_parameter(self.backend, &name, property == "rawValue", value)
    }
}

static PAGE_STATE: std::sync::Mutex<BTreeMap<String, bool>> = std::sync::Mutex::new(BTreeMap::new());

fn state_key(backend: &dyn Backend, state: &str) -> String {
    let vehicle = crate::read::object(&backend.get("vehicle.id")).get("value").cloned().unwrap_or(Value::Null);
    format!("{vehicle}#{state}")
}

pub const PAGE_OPENED: &str = "setup.pageOpened";

pub fn page_opened(backend: &dyn Backend, args: &str) -> Value {
    let Some(opened) = serde_json::from_str::<Value>(args).ok().and_then(|a| a.get(0)?.as_str().map(str::to_string)) else {
        return json!({ "ok": false, "reason": "setup.pageOpened takes the page name" });
    };
    let px4 = px4(backend);
    let merged = (opened == "Flight Modes" && !px4).then_some(SIMPLE_MODES);
    let pages: Vec<&str> = std::iter::once(opened.as_str()).chain(merged).collect();
    let keys: Vec<String> = pages
        .iter()
        .filter_map(|page| config(backend, page, px4))
        .flat_map(|c| c["state"].as_object().map(|o| o.keys().cloned().collect::<Vec<_>>()).unwrap_or_default())
        .map(|state| state_key(backend, &state))
        .collect();
    page_state().retain(|key, _| !keys.contains(key));
    let ready = !crate::qthost::present() && crate::read::object(&backend.get("vehicle.parameterManager.parametersReady"))["value"] == true;
    let missing = if ready { missing_parameters(backend, &pages, px4) } else { vec![] };
    if !missing.is_empty() {
        crate::noticeboard::post(crate::noticeboard::MESSAGE, "", &missing_parameters_text(&missing));
    }
    json!({ "ok": true })
}

thread_local! {
    static UNMET: std::cell::RefCell<Option<Vec<String>>> = const { std::cell::RefCell::new(None) };
}

const MISSING_PARAM_COMPONENT: u8 = 1;

fn missing_parameters(backend: &dyn Backend, pages: &[&str], px4: bool) -> Vec<String> {
    let configs: Vec<Value> = pages.iter().filter_map(|page| config(backend, page, px4)).collect();
    let required: Vec<String> = configs
        .iter()
        .flat_map(|c| {
            let scope = scope_for(backend, c);
            c["params"]
                .as_object()
                .into_iter()
                .flatten()
                .filter(|(_, p)| flag(p, "required") && !flag(p, "existsOnly"))
                .filter_map(|(_, p)| p["name"].as_str().map(str::to_string))
                .filter(|name| !scope.exists(name))
                .collect::<Vec<_>>()
        })
        .collect();
    UNMET.set(Some(vec![]));
    pages.iter().for_each(|p| {
        page(backend, p, px4);
    });
    let evaluated = UNMET.take().unwrap_or_default();
    required.into_iter().chain(evaluated).fold(vec![], |seen, name| if seen.contains(&name) { seen } else { seen.into_iter().chain(std::iter::once(name)).collect() })
}

fn missing_parameters_text(names: &[String]) -> String {
    let listed = names.iter().map(|name| format!("{MISSING_PARAM_COMPONENT}:{name}")).collect::<Vec<_>>().join(", ");
    format!("Parameters are missing from firmware. You may be running a version of firmware which is not fully supported or your firmware has a bug in it. Missing params: {listed}")
}

fn page_state() -> std::sync::MutexGuard<'static, BTreeMap<String, bool>> {
    PAGE_STATE.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn strict_equal(a: &Val, b: &Val) -> bool {
    match (a, b) {
        (Val::Num(x), Val::Num(y)) => x == y,
        _ => a == b,
    }
}

fn write_parameter(backend: &dyn Backend, name: &str, raw: bool, value: f64) -> Result<(), String> {
    write_checked(backend, name, raw, value, true)
}

fn set_parameter(backend: &dyn Backend, name: &str, raw: bool, value: f64) -> Result<(), String> {
    write_checked(backend, name, raw, value, false)
}

fn write_checked(backend: &dyn Backend, name: &str, raw: bool, value: f64, checked: bool) -> Result<(), String> {
    let path = parameter_path(name);
    let payload = match checked {
        true => json!({ "value": value }),
        false => json!({ "value": value, "force": true }),
    }
    .to_string();
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
                        (true, true) => title.replace("{index}", &label),
                        (true, false) => title.replace(" {index}", "").replace("{index} ", "").replace("{index}", ""),
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

fn channels(scope: &Scope, template: &str) -> Vec<usize> {
    (1..=32).take_while(|n| scope.exists(&template.replace('#', &n.to_string()))).collect()
}

fn listed_channels(scope: &Scope, control: &Value) -> Vec<usize> {
    let first = control["firstChannel"].as_u64().map_or(1, |n| n as usize);
    let last = control["lastChannel"].as_u64().map_or(usize::MAX, |n| n as usize);
    channels(scope, control["channelParam"].as_str().unwrap_or_default()).into_iter().filter(|n| (first..=last).contains(n)).collect()
}

fn offered_channels(scope: &Scope, control: &Value) -> Vec<usize> {
    let shown = control["lastChannelExpr"].as_str().and_then(|expr| scope.eval_text(expr).number()).map_or(usize::MAX, |n| n as usize);
    listed_channels(scope, control).into_iter().filter(|n| *n <= shown).collect()
}

const AUTOTUNE_SWITCH_OPTION: f64 = 17.0;

fn as_shipped(control: Value) -> Value {
    match (control["control"].as_str(), control["component"].as_str()) {
        (Some("component"), Some("APMAutoTuneChannelSelector")) => json!({
            "control": "channelFunction",
            "label": "Channel for AutoTune switch:",
            "channelParam": "RC#_OPTION",
            "functionValue": AUTOTUNE_SWITCH_OPTION,
            "firstChannel": 7,
            "lastChannel": 12,
            "noneLabel": "None",
            "exclusive": true,
        }),
        _ => control,
    }
}

fn channel_for(scope: &Scope, template: &str, function: f64) -> Option<usize> {
    channels(scope, template).into_iter().find(|n| scope.fact_member(&template.replace('#', &n.to_string()), "rawValue").number() == Some(function))
}

fn channel_param(control: &Value, scope: &Scope) -> Option<String> {
    let of = control.get("channelOf")?;
    let template = of["channelParam"].as_str()?;
    let channel = channel_for(scope, template, of["functionValue"].as_f64()?)?;
    let prefix = template.split('#').next().unwrap_or_default();
    Some(format!("{prefix}{channel}_{}", control["param"].as_str().unwrap_or_default()))
}

fn row_path(page: &str, id: &str) -> String {
    format!("{ROW_PREFIX}{page}).{id}")
}

fn labelled(mut decoded: Value, control: &Value, enabled: bool) -> Value {
    if !control["label"].is_null() {
        decoded["label"] = control["label"].clone();
    }
    if let Some(zero) = control["labelWhenZero"].as_str().filter(|_| decoded["value"].as_f64() == Some(0.0)) {
        decoded["label"] = json!(zero);
    }
    let range = match control["control"].as_str() {
        Some("factslider") => control["sliderFrom"].as_f64().zip(control["sliderTo"].as_f64()),
        Some("slider") => control["sliderMin"].as_f64().or(decoded["minimum"].as_f64()).zip(control["sliderMax"].as_f64().or(decoded["maximum"].as_f64())),
        _ => None,
    };
    let range = match (control["sliderInMetres"].as_bool(), crate::units::cooking("m")) {
        (Some(true), Some(cooked)) => range.map(|(from, to)| ((cooked.shown)(from), (cooked.shown)(to))),
        _ => range,
    };
    if let Some((from, to)) = range {
        decoded["slider"] = json!({ "from": from, "to": to, "step": control["majorTickStepSize"].as_f64(), "decimals": control["decimalPlaces"].as_i64() });
    }
    if control["firstEntryIsAll"] == true {
        decoded["firstEntryIsAll"] = json!(true);
    }
    if let Some(description) = control["description"].as_str() {
        decoded["description"] = json!(description);
    }
    if !enabled {
        decoded["enabled"] = json!(false);
        decoded["disabledReason"] = json!("Not in use with the choice above");
    }
    decoded
}

fn listed_values(mut row: Value, control: &Value, fact: &Value) -> Value {
    let Some(entries) = control["enumValues"].as_array() else {
        return row;
    };
    let raw = fact.get("rawValue").or(fact.get("value")).cloned().unwrap_or(Value::Null);
    let chosen = entries.iter().find(|e| e["value"].as_f64().is_some_and(|v| raw.as_f64() == Some(v))).or(entries.first());
    row["control"] = json!("choice");
    row["options"] = entries.iter().map(|e| json!({ "label": e["label"], "raw": crate::control::raw_text(&e["value"]) })).collect();
    row["display"] = chosen.map_or(Value::Null, |e| e["label"].clone());
    row
}

fn rows(scope: &Scope, page: &str, id: &str, control: &Value) -> Vec<Value> {
    let calculator = (control["control"] == "dialogButton")
        .then(|| {
            let index = scope.eval_text(control["dialogButton"]["dialogParams"]["batteryIndex"].as_str().unwrap_or("0")).number().unwrap_or(0.0) as usize;
            let param = control["param"].as_str().map(|p| scope.full_name(p)).unwrap_or_default();
            crate::powercalc::calculator(control["dialogButton"]["dialogComponent"].as_str().unwrap_or_default(), index, &param)
        })
        .flatten();
    let indent = flag(control, "indent");
    let built: Vec<Value> = control_rows(scope, page, id, control)
        .into_iter()
        .map(|mut row| {
            if indent {
                row["indent"] = json!(true);
            }
            row
        })
        .collect();
    match calculator {
        Some(calculator) => built.into_iter().map(|mut row| {
            row["calculator"] = calculator.clone();
            row
        }).collect(),
        None => built,
    }
}

fn control_rows(scope: &Scope, page: &str, id: &str, control: &Value) -> Vec<Value> {
    let shipped = as_shipped(control.clone());
    let control = &shipped;
    if !scope.shown(control, "showWhen") {
        return vec![];
    }
    let channel_gate = control.get("enableWhenChannel").is_none_or(|of| {
        of["channelParam"].as_str().zip(of["functionValue"].as_f64()).is_some_and(|(template, function)| channel_for(scope, template, function).is_some())
    });
    let enabled = scope.shown(control, "enableWhen") && channel_gate;
    let kind = control["control"].as_str().unwrap_or("");
    let label = control["label"].clone();
    let path = row_path(page, id);
    let name = control["param"].as_str().map_or_else(|| format!("{page}.{id}"), |param| scope.full_name(param));
    if kind == "label" {
        return vec![json!({ "control": "label", "name": name, "label": label, "warning": flag(control, "warning"), "smallFont": flag(control, "smallFont"), "enabled": enabled, "path": path })];
    }
    if kind == "button" {
        return vec![json!({ "control": "button", "name": name, "label": label, "enabled": enabled, "path": path })];
    }
    if kind == "dialogButton" && control["dialogButton"]["dialogComponent"] == "ESCCalibrationDialog" {
        return vec![json!({ "control": "dialog", "name": name, "label": control["dialogButton"]["text"], "dialog": "escCalibration", "enabled": enabled, "path": path })];
    }
    if kind == "channelFunction" {
        let listed = listed_channels(scope, control);
        let offered = offered_channels(scope, control);
        let none = control["noneLabel"].as_str().unwrap_or("Disabled").to_string();
        let current = control["functionValue"].as_f64().and_then(|f| listed.iter().copied().find(|n| scope.fact_member(&control["channelParam"].as_str().unwrap_or_default().replace('#', &n.to_string()), "rawValue").number() == Some(f)));
        let options: Vec<Value> = std::iter::once(none.clone())
            .chain(offered.iter().map(|n| format!("Channel {n}")))
            .enumerate()
            .map(|(i, label)| json!({ "label": label, "raw": i.to_string() }))
            .collect();
        return match listed.is_empty() {
            true => vec![],
            false => vec![json!({
                "control": "choice",
                "name": name,
                "label": label,
                "options": options,
                "display": current.map_or(none, |n| format!("Channel {n}")),
                "enabled": enabled,
                "path": path,
            })],
        };
    }
    let (fact_path, fact) = match (control["param"].as_str(), control["setting"].as_str()) {
        (Some(_), _) if control.get("channelOf").is_some() => match channel_param(control, scope) {
            Some(name) => (parameter_path(&name), scope.fact(&name)),
            None => (String::new(), None),
        },
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
        ("toggleCheckbox", None) if control["optional"].as_bool() == Some(true) && control.get("param").is_some() => vec![],
        ("toggleCheckbox", _) => {
            let checked = scope.eval_text(control["toggleCheckbox"]["checked"].as_str().unwrap_or("false")).truthy();
            vec![json!({ "control": "toggle", "name": name, "label": label, "value": checked, "enabled": enabled, "path": path })]
        }
        (_, None) => vec![],
        ("bitmaskCheckbox", Some(fact)) => {
            let raw = fact.get("rawValue").or(fact.get("value")).and_then(Value::as_f64).unwrap_or(0.0) as i64;
            let bit = control["bitMask"].as_i64().unwrap_or(0);
            vec![json!({ "control": "toggle", "name": name, "label": label, "value": raw & bit != 0, "enabled": enabled, "path": path })]
        }
        ("radiogroup", Some(_)) => {
            let options: Vec<&Value> = control["options"].as_array().map(|o| o.iter().collect()).unwrap_or_default();
            let chosen = options.iter().find(|o| scope.eval_text(o["checked"].as_str().unwrap_or("false")).truthy());
            vec![json!({
                "control": "choice",
                "name": name,
                "label": label,
                "options": options.iter().enumerate().map(|(i, o)| json!({ "label": o["label"], "raw": i.to_string() })).collect::<Vec<_>>(),
                "display": chosen.map_or(Value::Null, |o| o["label"].clone()),
                "enabled": enabled,
                "path": path,
            })]
        }
        ("factslider", Some(fact)) if control.get("linkedParams").is_some() => vec![labelled(decode(&fact, &path), control, enabled)],
        (_, Some(fact)) => {
            let mut row = listed_values(labelled(decode(&fact, &fact_path), control, enabled), control, &fact);
            if control["enumValues"].is_array() {
                row["path"] = json!(path);
            }
            match control.get("enableCheckbox") {
                None => vec![row],
                Some(toggle) => {
                    let checked = scope.eval_text(toggle["checked"].as_str().unwrap_or("false")).truthy();
                    let switch_label = toggle.get("label").unwrap_or(&label).clone();
                    let gated = match checked {
                        true => row,
                        false => {
                            let mut off = labelled(row, control, false);
                            if enabled {
                                let named = toggle.get("label").is_none().then(|| label.as_str()).flatten();
                                off["disabledReason"] = json!(format!("Turn on {} to edit", named.unwrap_or("the switch above")));
                            }
                            off
                        }
                    };
                    vec![json!({ "control": "toggle", "name": format!("{name}.enable"), "label": switch_label, "value": checked, "enabled": enabled, "path": format!("{path}.enable") }), gated]
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
                    Some(_) => json!({ "control": "toggle", "name": name, "label": heading, "value": false, "path": row_path(page, &format!("{section_index}.{index}.disabled")) }),
                    None => labelled(decode(&fact, &parameter_path(&name)), &json!({ "label": heading }), true),
                }
            })
        })
        .collect();
    (!controls.is_empty()).then(|| json!({ "title": companion["heading"], "note": "", "keywords": companion["heading"].as_str().map(str::to_lowercase).into_iter().collect::<Vec<_>>(), "controls": controls }))
}

fn search_terms(section: &Value) -> Vec<String> {
    let strs = |v: &Value| v.as_str().map(str::to_lowercase);
    let controls = section["controls"].as_array().cloned().unwrap_or_default();
    std::iter::once(&section["title"])
        .chain(section["keywords"].as_array().into_iter().flatten())
        .chain(controls.iter().flat_map(|c| [&c["label"], &c["param"]]))
        .filter_map(strs)
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn section_image(section: &Value) -> Value {
    section["image"].as_str().and_then(|path| path.rsplit('/').next()).filter(|leaf| !leaf.is_empty()).map_or(Value::Null, |leaf| json!(leaf))
}

pub fn page(backend: &dyn Backend, page: &str, px4: bool) -> Value {
    let Some(config) = config(backend, page, px4) else { return crate::read::refused(&format!("{page} has no VehicleConfig definition for this firmware")) };
    let base = scope_for(backend, &config);
    let sections: Vec<Value> = config["sections"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .enumerate()
        .flat_map(|(section_index, section)| {
            let shown: Vec<Value> = instances(&base, section)
                    .into_iter()
                    .filter_map(|instance| {
                        let scope = base.with(instance.repeat, BTreeMap::from([("_rawIndex".to_string(), Val::Num((instance.index + 1) as f64))]));
                        if !scope.shown(section, "showWhen") {
                            return None;
                        }
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
                        Some(json!({ "title": instance.heading, "note": "", "image": section_image(section), "keywords": search_terms(section), "controls": controls }))
                    })
                    .collect();
            shown.into_iter().chain(disabled_companion(&base, page, section_index, section))
        })
        .filter(|s| s["controls"].as_array().is_some_and(|c| !c.is_empty()))
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
    let config = config(backend, &at.page, px4(backend)).ok_or_else(|| format!("{} has no definition for this firmware", at.page))?;
    let section = config["sections"].get(at.section).cloned().ok_or("That section is gone.")?;
    let repeat = match section.get("repeat") {
        None => None,
        Some(_) => instances(&scope_for(backend, &config), &section).into_iter().nth(at.instance).ok_or("That battery is no longer reported.")?.repeat,
    };
    let control = as_shipped(at.control.and_then(|i| section["controls"].get(i).cloned()).unwrap_or(section));
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
                (Some(name), Some(enabled), true) => set_parameter(backend, &name, false, enabled),
                (Some(_), _, false) => Ok(()),
                _ => Err("That battery is no longer reported.".to_string()),
            }
        }
        ("enable", ..) => {
            let locals = BTreeMap::from([("enableCheckBoxChecked".to_string(), Val::Bool(checked))]);
            scope.with(resolved.repeat.clone(), locals).assign(control["enableCheckbox"]["onClicked"].as_str().unwrap_or_default())
        }
        (_, "button", _) if scope.shown(control, "enableWhen") => scope.assign(control["button"]["onClicked"].as_str().unwrap_or_default()),
        (_, "button", _) => Err("That button is not available now.".to_string()),
        (_, "toggleCheckbox", _) => scope.assign(control["toggleCheckbox"][if checked { "onChecked" } else { "onUnchecked" }].as_str().unwrap_or_default()),
        (_, "bitmaskCheckbox", _) => {
            let name = scope.full_name(control["param"].as_str().unwrap_or_default());
            let raw = scope.fact_member(&name, "rawValue").number().unwrap_or(0.0) as i64;
            let bit = control["bitMask"].as_i64().unwrap_or(0);
            write_parameter(backend, &name, true, (if checked { raw | bit } else { raw & !bit }) as f64)
        }
        (_, "channelFunction", true) => {
            let template = control["channelParam"].as_str().unwrap_or_default();
            let function = control["functionValue"].as_f64().unwrap_or(f64::NAN);
            let chosen = asked.as_u64().or_else(|| asked.as_str().and_then(|s| s.parse().ok())).map(|i| i as usize);
            let listed = listed_channels(&scope, control);
            let offered = offered_channels(&scope, control);
            let param = |n: usize| template.replace('#', &n.to_string());
            let holding: Vec<usize> = listed.iter().copied().filter(|n| scope.fact_member(&param(*n), "rawValue").number() == Some(function)).collect();
            let clear = |keep: Option<usize>| holding.iter().filter(|n| Some(**n) != keep).take(if flag(control, "exclusive") { usize::MAX } else { 1 }).try_for_each(|n| write_parameter(backend, &param(*n), true, 0.0));
            match chosen.map(|i| i.checked_sub(1).map(|at| offered.get(at).copied())) {
                Some(None) => clear(None),
                Some(Some(Some(n))) => match flag(control, "exclusive") {
                    true => clear(Some(n)).and_then(|()| write_parameter(backend, &param(n), true, function)),
                    false => write_parameter(backend, &param(n), true, function),
                },
                _ => Err("Choose one of the listed channels.".to_string()),
            }
        }
        (_, _, true) if control["enumValues"].is_array() => {
            let name = scope.full_name(control["param"].as_str().unwrap_or_default());
            let index = asked.as_u64().or_else(|| asked.as_str().and_then(|s| s.parse().ok()));
            match index.and_then(|i| control["enumValues"].get(i as usize)).and_then(|e| e["value"].as_f64()) {
                Some(v) => set_parameter(backend, &name, true, v),
                None => Err("Choose one of the listed options.".to_string()),
            }
        }
        (_, "radiogroup", true) => {
            let name = scope.full_name(control["param"].as_str().unwrap_or_default());
            let option = asked.as_u64().or_else(|| asked.as_str().and_then(|s| s.parse().ok())).and_then(|i| control["options"].get(i as usize));
            if let Some(statement) = option.and_then(|o| o["onSelected"].as_str()) {
                return answer(scope.assign(statement));
            }
            match option.and_then(|o| scope.eval_text(o["value"].as_str().unwrap_or_default()).number()) {
                Some(v) => set_parameter(backend, &name, flag(control, "raw"), v),
                None => Err("Choose one of the listed options.".to_string()),
            }
        }
        (_, "factslider", _) => {
            let name = scope.full_name(control["param"].as_str().unwrap_or_default());
            match number {
                None => Err(crate::factwrite::INVALID_NUMBER.to_string()),
                Some(v) => write_parameter(backend, &name, false, v).and_then(|()| {
                    let locals = BTreeMap::from([("value".to_string(), Val::Num(v))]);
                    let linked = scope.with(resolved.repeat.clone(), locals);
                    control["linkedParams"].as_object().cloned().unwrap_or_default().iter().try_for_each(|(other, expression)| {
                        let computed = linked.eval_text(expression.as_str().unwrap_or("value")).number().ok_or_else(|| format!("{other} could not be computed"))?;
                        set_parameter(backend, other, true, computed)
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

    #[test]
    fn plain_sliders_and_first_entry_all_bitmasks_carry_their_qml_properties() {
        let slider = labelled(json!({}), &json!({ "control": "slider", "sliderMin": 0, "sliderMax": 10 }), true);
        assert_eq!((slider["slider"]["from"].as_f64(), slider["slider"]["to"].as_f64()), (Some(0.0), Some(10.0)), "FactTextFieldSlider draws sliderMin..sliderMax");
        assert_eq!(labelled(json!({}), &json!({ "control": "slider" }), true)["slider"], Value::Null, "with no range there is no slider");
        let bounded = labelled(json!({ "minimum": 0.0, "maximum": 10000.0 }), &json!({ "control": "slider" }), true);
        assert_eq!((bounded["slider"]["from"].as_f64(), bounded["slider"]["to"].as_f64()), (Some(0.0), Some(10000.0)), "allowUsingMinMax: a plain slider falls back to the fact's own bounds");
        let mixed = labelled(json!({ "minimum": -1.0, "maximum": 15.0 }), &json!({ "control": "slider", "sliderMax": 5 }), true);
        assert_eq!((mixed["slider"]["from"].as_f64(), mixed["slider"]["to"].as_f64()), (Some(-1.0), Some(5.0)), "each end falls back on its own");
        assert_eq!(labelled(json!({ "minimum": 0.0, "maximum": 1.0 }), &json!({ "control": "factslider" }), true)["slider"], Value::Null, "factslider has no such fallback");
        assert_eq!(labelled(json!({}), &json!({ "control": "bitmask", "firstEntryIsAll": true }), true)["firstEntryIsAll"], true);
        assert_eq!(labelled(json!({}), &json!({ "control": "bitmask" }), true)["firstEntryIsAll"], Value::Null);
    }
    use std::cell::RefCell;

    #[test]
    fn ekf3_logging_lists_the_four_choices_the_config_names() {
        let fake = Fake::new(&[("EK3_LOG_LEVEL", 2.0)]);
        let served = page(&fake, "Logging", false);
        let row = served["sections"].as_array().unwrap().iter().flat_map(|s| s["controls"].as_array().cloned().unwrap_or_default()).find(|r| r["name"] == "EK3_LOG_LEVEL").unwrap();
        assert_eq!(row["control"], "choice");
        let labels: Vec<&str> = row["options"].as_array().unwrap().iter().filter_map(|o| o["label"].as_str()).collect();
        assert_eq!(labels, ["Full logging", "XKF4 scaled innovations only", "XKF4 and GSF", "Disabled"]);
        assert_eq!(row["options"][3]["raw"], "3");
        assert_eq!(row["display"], "XKF4 and GSF");
        assert_eq!(row["label"], "EKF3 logging verbosity");
        assert_eq!(write(&fake, &format!("{}{ENUM_INDEX}", row["path"].as_str().unwrap()), r#"{"value":3}"#)["ok"], true, "the combo writes the chosen option's value, the parameter itself lists none");
        assert_eq!(fake.writes.borrow().last(), Some(&("EK3_LOG_LEVEL".to_string(), 3.0)));
    }

    #[test]
    fn sections_name_their_icon_by_file() {
        assert_eq!(section_image(&json!({ "image": "/qmlimages/Battery.svg" })), "Battery.svg");
        assert_eq!(section_image(&json!({})), Value::Null);
        let fake = Fake::new(&[("BATT_MONITOR", 4.0), ("BATT_CAPACITY", 5000.0)]);
        let served = page(&fake, "Power", false);
        assert!(served["sections"].as_array().unwrap().iter().any(|s| s["image"] == "Battery.svg"), "{served}");
    }

    #[test]
    fn indented_help_rows_say_so() {
        let fake = Fake::new(&[]);
        let config = json!({});
        let scope = scope_for(&fake, &config);
        let help = rows(&scope, "P", "0.0", &json!({ "control": "label", "label": "Why", "indent": true, "smallFont": true }));
        assert_eq!((help[0]["indent"].clone(), help[0]["smallFont"].clone()), (json!(true), json!(true)));
        let plain = rows(&scope, "P", "0.1", &json!({ "control": "label", "label": "Why" }));
        assert_eq!((plain[0]["indent"].clone(), plain[0]["smallFont"].clone()), (Value::Null, json!(false)));
    }

    #[test]
    fn a_value_off_the_list_shows_the_first_entry_as_the_combo_does() {
        let row = listed_values(json!({}), &json!({ "enumValues": [{ "value": 0, "label": "A" }, { "value": 1, "label": "B" }] }), &json!({ "rawValue": 9 }));
        assert_eq!(row["display"], "A");
        assert_eq!(listed_values(json!({ "control": "number" }), &json!({}), &json!({}))["control"], "number");
    }

    struct Fake {
        params: RefCell<BTreeMap<String, f64>>,
        multi_rotor: bool,
        px4: bool,
        sub: bool,
        writes: RefCell<Vec<(String, f64)>>,
    }

    impl Fake {
        fn new(params: &[(&str, f64)]) -> Self {
            Fake { params: RefCell::new(params.iter().map(|(n, v)| (n.to_string(), *v)).collect()), multi_rotor: true, px4: false, sub: false, writes: RefCell::new(vec![]) }
        }
    }

    fn name_of(path: &str) -> Option<&str> {
        path.strip_prefix("vehicle.parameterManager.getParameter(-1,").and_then(|p| p.split(')').next())
    }

    impl Backend for Fake {
        fn get(&self, path: &str) -> String {
            if path == "vehicle.id" {
                return json!({ "kind": "value", "value": self.params.borrow().get("VEHICLE_ID").copied().unwrap_or(1.0) }).to_string();
            }
            match name_of(path).and_then(|n| self.params.borrow().get(n).map(|v| (n.to_string(), *v))) {
                Some((name, value)) => json!({ "kind": "fact", "name": name, "value": value, "rawValue": value, "valueString": value.to_string() }),
                None => json!({ "kind": "null" }),
            }
            .to_string()
        }
        fn get_fields(&self, _p: &str, _f: &str) -> String {
            json!({ "kind": "object", "multiRotor": self.multi_rotor, "fixedWing": !self.multi_rotor, "px4Firmware": self.px4, "sub": self.sub }).to_string()
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
    fn px4_power_rows_carry_their_calculators_for_the_right_battery() {
        let mut fake = Fake::new(&[("BAT1_SOURCE", 0.0), ("BAT1_V_DIV", 10.0), ("BAT1_A_PER_V", 36.0), ("BAT2_SOURCE", 0.0), ("BAT2_V_DIV", 11.0)]);
        fake.px4 = true;
        let served = page(&fake, "Power", true);
        let rows: Vec<Value> = served["sections"].as_array().unwrap().iter().flat_map(|s| s["controls"].as_array().cloned().unwrap_or_default()).collect();
        let calculators: Vec<(String, u64)> = rows.iter().filter_map(|r| Some((r["calculator"]["param"].as_str()?.to_string(), r["calculator"]["batteryIndex"].as_u64()?))).collect();
        assert!(calculators.contains(&("BAT1_V_DIV".to_string(), 1)), "{calculators:?}");
        assert!(calculators.contains(&("BAT1_A_PER_V".to_string(), 1)));
        assert!(calculators.contains(&("BAT2_V_DIV".to_string(), 2)));
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
                    strings(c, &["showWhen", "enableWhen", "lastChannelExpr"])
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
    fn a_choice_the_page_makes_is_written_unchecked_as_fact_value_assignment_is() {
        struct Ranged(RefCell<Vec<String>>);
        impl Backend for Ranged {
            fn get(&self, _p: &str) -> String { json!({ "kind": "fact", "name": "RTL_ALT", "value": 1500, "rawValue": 1500, "typeIsInteger": true, "min": 200, "max": 32767, "minString": "200", "maxString": "32767" }).to_string() }
            fn get_fields(&self, _p: &str, _f: &str) -> String { String::new() }
            fn set(&self, _p: &str, value: &str) -> String {
                self.0.borrow_mut().push(value.to_string());
                json!({ "ok": true }).to_string()
            }
            fn invoke(&self, _p: &str, _a: &str) -> String { String::new() }
            fn watch(&self, _p: &[String]) {}
        }
        let ranged = Ranged(RefCell::new(vec![]));
        assert!(write_parameter(&ranged, "RTL_ALT", false, 0.0).is_err(), "a value the operator types is still held to the range");
        assert_eq!(set_parameter(&ranged, "RTL_ALT", false, 0.0), Ok(()), "Return at current altitude sets RTL_ALT 0 below its minimum, as Fact::setCookedValue does without validating");
        assert_eq!(ranged.0.borrow().len(), 1);
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
    fn a_section_of_only_labels_is_still_shown_like_the_generated_qml() {
        let fake = Fake::new(&[("FS_GCS_ENABLE", 1.0), ("FS_GCS_TIMEOUT", 5.0), ("FS_THR_ENABLE", 1.0), ("FS_THR_VALUE", 975.0)]);
        let failsafes = page(&fake, "Failsafes", false);
        let rc = failsafes["sections"].as_array().unwrap().iter().find(|s| s["title"].as_str().is_some_and(|t| t.contains("RC"))).cloned();
        assert!(rc.is_some(), "the RC failsafe section stays with only its label when FS_OPTIONS is missing: {failsafes}");
    }

    #[test]
    fn reopening_the_page_derives_custom_again_like_a_new_controller() {
        let fake = Fake::new(&[("SIMPLE", 0.0), ("SUPER_SIMPLE", 0.0), ("VEHICLE_ID", 7.0)]);
        let rows = || page(&fake, SIMPLE_MODES, false)["sections"][0]["controls"].as_array().unwrap().clone();
        let mode = rows()[0].clone();
        assert_eq!(write(&fake, &format!("{}{ENUM_INDEX}", mode["path"].as_str().unwrap()), r#"{"value":3}"#)["ok"], true);
        assert_eq!(rows()[0]["display"], "Custom");
        assert_eq!(page_opened(&fake, r#"["Power"]"#)["ok"], true);
        assert_eq!(rows()[0]["display"], "Custom", "another page's form leaves Flight Modes' state alone");
        assert_eq!(page_opened(&fake, r#"["Flight Modes"]"#)["ok"], true);
        assert_eq!(rows()[0]["display"], "Off");
    }

    #[test]
    fn opening_a_page_reports_its_required_and_looked_up_parameters_that_the_firmware_lacks() {
        let mut fake = Fake::new(&[("GF_MAX_HOR_DIST", 0.0), ("COM_DISARM_LAND", 2.0), ("RTL_LAND_DELAY", 0.0)]);
        fake.px4 = true;
        let missing = missing_parameters(&fake, &["Safety"], true);
        assert_eq!(missing[..2], ["CP_DIST".to_string(), "GF_MAX_VER_DIST".to_string()], "required params first, in config order: {missing:?}");
        assert_eq!(missing.iter().filter(|n| *n == "CP_DIST").count(), 1, "each name once");
        let present = Fake::new(&[("CP_DIST", 1.0), ("GF_MAX_HOR_DIST", 0.0), ("GF_MAX_VER_DIST", 0.0), ("COM_DISARM_LAND", 2.0), ("RTL_LAND_DELAY", 0.0)]);
        assert!(missing_parameters(&present, &["Safety"], true).iter().all(|n| !["CP_DIST", "GF_MAX_VER_DIST"].contains(&n.as_str())));
        let mut bare = Fake::new(&[]);
        bare.px4 = true;
        let battery = missing_parameters(&bare, &[BATTERY_SETTINGS], true);
        assert!(["COM_LOW_BAT_ACT", "BAT_LOW_THR", "BAT_CRIT_THR", "BAT_EMERGEN_THR"].iter().all(|n| battery.contains(&n.to_string())), "PX4BatteryIndicator looks every one up unguarded: {battery:?}");
        let config = json!({});
        let scope = scope_for(&fake, &config);
        UNMET.set(Some(vec![]));
        scope.eval_text("controller.getParameterFact(-1, \"NOPE\", false) || controller.getParameterFact(-1, \"GONE\")");
        assert_eq!(UNMET.take(), Some(vec!["GONE".to_string()]), "reportMissing false stays quiet");
        assert_eq!(
            missing_parameters_text(&["CP_DIST".into(), "GF_MAX_VER_DIST".into()]),
            "Parameters are missing from firmware. You may be running a version of firmware which is not fully supported or your firmware has a bug in it. Missing params: 1:CP_DIST, 1:GF_MAX_VER_DIST"
        );
    }

    #[test]
    fn sections_carry_the_generated_qml_search_terms() {
        let fake = Fake::new(&[("FS_GCS_ENABLE", 1.0), ("FS_GCS_TIMEOUT", 5.0), ("FS_OPTIONS", 0.0), ("FS_THR_ENABLE", 1.0), ("FS_THR_VALUE", 975.0)]);
        let failsafes = page(&fake, "Failsafes", false);
        let gcs = failsafes["sections"].as_array().unwrap().iter().find(|s| s["title"] == "Ground Station Failsafe").unwrap();
        let terms: Vec<&str> = gcs["keywords"].as_array().unwrap().iter().filter_map(Value::as_str).collect();
        ["ground station failsafe", "heartbeat", "fs_gcs_timeout", "timeout"].iter().for_each(|t| assert!(terms.contains(t), "{t} in {terms:?}"));
    }

    #[test]
    fn every_row_is_named_and_labelled() {
        let fake = Fake::new(&[("FS_GCS_ENABLE", 1.0), ("FS_GCS_TIMEOUT", 5.0), ("FS_OPTIONS", 0.0), ("FS_THR_ENABLE", 1.0), ("FS_THR_VALUE", 975.0), ("ARMING_CHECK", 1.0), ("FENCE_ENABLE", 1.0), ("FENCE_TYPE", 3.0), ("RTL_ALT_M", 15.0), ("RTL_LOIT_TIME", 5.0)]);
        ["Failsafes", "Flight Safety"].iter().for_each(|name| {
            page(&fake, name, false)["sections"].as_array().unwrap().iter().flat_map(|s| s["controls"].as_array().unwrap().clone()).for_each(|c| {
                assert!(c["name"].as_str().is_some_and(|n| !n.is_empty()), "{name}: {c}");
                assert!(c["label"].as_str().is_some_and(|l| !l.is_empty()), "{name}: {c}");
            });
        });
    }

    #[test]
    fn the_gimbal_page_follows_the_mount_and_its_channels() {
        let fake = Fake::new(&[("MNT1_TYPE", 1.0), ("MNT1_DEFLT_MODE", 3.0), ("MNT1_NEUTRAL_Y", 0.0), ("MNT1_RC_RATE", 0.0), ("RC1_OPTION", 0.0), ("RC2_OPTION", 213.0), ("SERVO1_FUNCTION", 0.0), ("SERVO2_FUNCTION", 7.0), ("SERVO2_REVERSED", 0.0), ("SERVO2_MIN", 1100.0), ("SERVO2_MAX", 1900.0)]);
        let served = page(&fake, "Gimbal", false);
        let titles: Vec<&str> = served["sections"].as_array().unwrap().iter().map(|s| s["title"].as_str().unwrap()).collect();
        assert_eq!(titles[0], "Gimbal", "one mount carries no number, as QGC hides the tab bar");
        assert!(titles.contains(&"Gimbal servo controlled gimbal"), "{titles:?}");
        let rows: Vec<Value> = served["sections"].as_array().unwrap().iter().flat_map(|s| s["controls"].as_array().unwrap().clone()).collect();
        let pitch_rc = rows.iter().find(|r| r["label"] == "Pitch" && r["control"] == "choice").unwrap();
        assert_eq!(pitch_rc["display"], "Channel 2", "the RC channel whose option is Mount1 pitch");
        assert!(rows.iter().any(|r| r["name"] == "SERVO2_MIN"), "servo rows follow the channel carrying the pitch function");
        assert!(!rows.iter().any(|r| r["label"] == "Yaw min PWM"), "an axis with no output channel has no servo rows");
        assert_eq!(write(&fake, &format!("{}{ENUM_INDEX}", pitch_rc["path"].as_str().unwrap()), r#"{"value":1}"#)["ok"], true);
        assert_eq!((fake.params.borrow()["RC1_OPTION"], fake.params.borrow()["RC2_OPTION"]), (213.0, 213.0), "choosing a channel sets its option; QGC leaves the old one to the operator");
        assert_eq!(write(&fake, &format!("{}{ENUM_INDEX}", pitch_rc["path"].as_str().unwrap()), r#"{"value":0}"#)["ok"], true);
        assert_eq!(fake.params.borrow()["RC1_OPTION"], 0.0, "Disabled clears the first channel carrying the function");
    }

    #[test]
    fn a_mount_that_needs_a_reboot_says_so() {
        let fake = Fake::new(&[("MNT1_TYPE", 1.0)]);
        let served = page(&fake, "Gimbal", false);
        let rows: Vec<Value> = served["sections"].as_array().unwrap().iter().flat_map(|s| s["controls"].as_array().unwrap().clone()).collect();
        assert!(rows.iter().any(|r| r["label"] == "Gimbal settings will be available after rebooting the vehicle."));
        assert_eq!(served["sections"].as_array().unwrap().len(), 1, "until the mount's parameters appear only its type is offered");
    }

    #[test]
    fn the_px4_flight_mode_menu_edits_rtl_altitude_and_the_geofence() {
        let fake = Fake { px4: true, ..Fake::new(&[("RTL_RETURN_ALT", 60.0), ("GF_ACTION", 1.0), ("GF_MAX_HOR_DIST", 0.0), ("GF_MAX_VER_DIST", 120.0)]) };
        let served = page(&fake, FLIGHT_MODE_SETTINGS, true);
        let rows: Vec<Value> = served["sections"].as_array().unwrap().iter().flat_map(|s| s["controls"].as_array().cloned().unwrap_or_default()).collect();
        let labels: Vec<&str> = rows.iter().filter_map(|r| r["label"].as_str()).collect();
        assert_eq!(labels, ["RTL Altitude", "Breach Action", "Max Distance", "Max Distance", "Max Altitude", "Max Altitude"], "PX4FlightModeIndicator pairs each fence limit with its checkbox");
        let distance = rows.iter().find(|r| r["label"] == "Max Distance" && r["control"] == "toggle").unwrap().clone();
        assert_eq!(distance["value"], false);
        assert_eq!(write(&fake, distance["path"].as_str().unwrap(), r#"{"value":true}"#)["ok"], true);
        assert_eq!(fake.params.borrow()["GF_MAX_HOR_DIST"], 1000.0, "switching the limit on starts it at the go-to distance limit");
    }

    #[test]
    fn simple_mode_sets_both_masks_and_custom_offers_each_slot() {
        let fake = Fake::new(&[("SIMPLE", 0.0), ("SUPER_SIMPLE", 0.0)]);
        let rows = |fake: &Fake| -> Vec<Value> { page(fake, SIMPLE_MODES, false)["sections"].as_array().unwrap().iter().flat_map(|s| s["controls"].as_array().cloned().unwrap_or_default()).collect() };
        let mode = rows(&fake)[0].clone();
        assert_eq!((mode["display"].as_str(), rows(&fake).len()), (Some("Off"), 1), "APMFlightModesComponent shows the per-slot checkboxes only in Custom");
        assert_eq!(write(&fake, &format!("{}{ENUM_INDEX}", mode["path"].as_str().unwrap()), r#"{"value":2}"#)["ok"], true);
        assert_eq!((fake.params.borrow()["SIMPLE"], fake.params.borrow()["SUPER_SIMPLE"]), (0.0, 63.0), "Super-Simple puts every slot in super simple");
        fake.params.borrow_mut().insert("SUPER_SIMPLE".into(), 4.0);
        let custom = rows(&fake);
        assert_eq!(custom[0]["display"], "Custom");
        assert_eq!(custom.len(), 13);
        let third = custom.iter().find(|r| r["label"] == "Flight Mode 3 Super-Simple").unwrap();
        assert_eq!(third["value"], true);

        let fake = Fake::new(&[("SIMPLE", 63.0), ("SUPER_SIMPLE", 0.0)]);
        page_opened(&fake, r#"["Flight Modes"]"#);
        let mode = rows(&fake)[0].clone();
        assert_eq!(write(&fake, &format!("{}{ENUM_INDEX}", mode["path"].as_str().unwrap()), r#"{"value":3}"#)["ok"], true);
        assert_eq!((fake.params.borrow()["SIMPLE"], fake.params.borrow()["SUPER_SIMPLE"]), (0.0, 0.0), "_updateSimpleParamsFromSimpleMode writes 0 and 0 for Custom, never Simple on slot 1");
        let custom = rows(&fake);
        assert_eq!((custom[0]["display"].as_str(), custom.len()), (Some("Custom"), 13), "and the page stays in Custom with every box unchecked");
        let other = Fake::new(&[("SIMPLE", 0.0), ("SUPER_SIMPLE", 0.0), ("VEHICLE_ID", 2.0)]);
        assert_eq!(rows(&other)[0]["display"], "Off", "another vehicle at 0/0 derives Off, like a fresh APMFlightModesComponentController");
        assert_eq!(write(&fake, &format!("{}{ENUM_INDEX}", mode["path"].as_str().unwrap()), r#"{"value":0}"#)["ok"], true);
        assert_eq!(rows(&fake)[0]["display"], "Off");
    }

    #[test]
    fn the_autotune_switch_moves_option_17_to_one_channel_from_7_to_12() {
        let options: Vec<(String, f64)> = (1..=16).map(|n| (format!("RC{n}_OPTION"), if n == 7 { 17.0 } else { 0.0 })).collect();
        let named: Vec<(&str, f64)> = options.iter().map(|(n, v)| (n.as_str(), *v)).chain([("AUTOTUNE_AXES", 7.0)]).collect();
        let fake = Fake::new(&named);
        let rows: Vec<Value> = page(&fake, "Tuning", false)["sections"].as_array().unwrap().iter().flat_map(|s| s["controls"].as_array().cloned().unwrap_or_default()).collect();
        let switch = rows.iter().find(|r| r["label"] == "Channel for AutoTune switch:").unwrap().clone();
        assert_eq!(switch["display"], "Channel 7");
        assert_eq!(switch["options"].as_array().unwrap().len(), 7, "None and channels 7 to 12, as APMAutoTuneChannelSelector offers");
        assert_eq!(write(&fake, &format!("{}{ENUM_INDEX}", switch["path"].as_str().unwrap()), r#"{"value":3}"#)["ok"], true);
        assert_eq!((fake.params.borrow()["RC7_OPTION"], fake.params.borrow()["RC9_OPTION"]), (0.0, 17.0), "the switch moves rather than being added to a second channel");
        assert_eq!(write(&fake, &format!("{}{ENUM_INDEX}", switch["path"].as_str().unwrap()), r#"{"value":0}"#)["ok"], true);
        assert_eq!(fake.params.borrow()["RC9_OPTION"], 0.0);
    }

    #[test]
    fn the_battery_dropdown_edits_the_failsafe_triggers() {
        let apm = Fake::new(&[("BATT_MONITOR", 4.0), ("BATT_FS_LOW_ACT", 2.0), ("BATT_LOW_VOLT", 10.5), ("BATT_FS_CRT_ACT", 1.0), ("BATT_CRT_VOLT", 9.8)]);
        let labels = |fake: &Fake, px4: bool| -> Vec<String> { page(fake, BATTERY_SETTINGS, px4)["sections"].as_array().unwrap().iter().flat_map(|s| s["controls"].as_array().cloned().unwrap_or_default()).filter_map(|c| c["label"].as_str().map(str::to_string)).collect() };
        assert_eq!(labels(&apm, false), ["Vehicle Action", "Voltage Trigger", "Vehicle Action", "Voltage Trigger"], "a trigger the firmware lacks is left out");
        assert!(labels(&Fake::new(&[("BATT_MONITOR", 0.0), ("BATT_FS_LOW_ACT", 2.0)]), false).is_empty(), "APMBatteryIndicator hides the failsafes with no battery monitor");
        let triggers = Fake::new(&[("BATT_MONITOR", 4.0), ("BATT_LOW_VOLT", 0.0), ("BATT_LOW_MAH", 1500.0)]);
        let rows: Vec<Value> = page(&triggers, BATTERY_SETTINGS, false)["sections"][0]["controls"].as_array().cloned().unwrap_or_default();
        assert_eq!(rows.iter().map(|r| r["label"].as_str().unwrap_or_default()).collect::<Vec<_>>(), ["Voltage Trigger - disabled", "mAh Trigger"], "a zero trigger says it is off, like APMBatteryIndicator's disabledString");
        assert_eq!((rows[0]["slider"]["from"].as_f64(), rows[0]["slider"]["to"].as_f64(), rows[1]["slider"]["to"].as_f64()), (Some(0.0), Some(100.0), Some(30000.0)), "BATT_LOW_VOLT and BATT_LOW_MAH carry no range in the metadata; the FactSliders pin 0..100 V and 0..30000 mAh");
        let px4 = Fake { px4: true, ..Fake::new(&[("COM_LOW_BAT_ACT", 3.0), ("BAT_LOW_THR", 0.15), ("BAT_CRIT_THR", 0.07), ("BAT_EMERGEN_THR", 0.05)]) };
        assert_eq!(labels(&px4, true), ["Vehicle Action", "Warning Level", "Critical Level", "Emergency Level"]);
    }

    #[test]
    fn the_status_dropdown_edits_the_comm_loss_failsafe() {
        let labels = |fake: &Fake, px4: bool| -> Vec<String> { page(fake, STATUS_SETTINGS, px4)["sections"].as_array().unwrap().iter().flat_map(|s| s["controls"].as_array().cloned().unwrap_or_default()).filter_map(|c| c["label"].as_str().map(str::to_string)).collect() };
        assert_eq!(labels(&Fake::new(&[("FS_GCS_ENABLE", 1.0), ("FS_GCS_TIMEOUT", 5.0)]), false), ["Vehicle Action", "Loss Timeout"], "APMMainStatusIndicator offers only what the build carries");
        assert_eq!(labels(&Fake { px4: true, ..Fake::new(&[("NAV_DLL_ACT", 2.0), ("COM_DL_LOSS_T", 10.0)]) }, true), ["Vehicle Action", "Loss Timeout"]);
    }

    #[test]
    fn lights_outputs_pick_a_channel_from_5_to_16_for_each_light() {
        let named: Vec<(String, f64)> = (1..=16).map(|n| (format!("SERVO{n}_FUNCTION"), if n == 9 { 59.0 } else { 0.0 })).collect();
        let fake = Fake::new(&named.iter().map(|(n, v)| (n.as_str(), *v)).chain([("JS_LIGHTS_STEPS", 4.0)]).collect::<Vec<_>>());
        let rows: Vec<Value> = page(&fake, "Lights", false)["sections"].as_array().unwrap().iter().flat_map(|s| s["controls"].as_array().cloned().unwrap_or_default()).collect();
        let lights1 = rows.iter().find(|r| r["label"] == "Lights 1").unwrap().clone();
        assert_eq!(lights1["display"], "Channel 9");
        assert_eq!(lights1["options"].as_array().unwrap().len(), 13, "Disabled and channels 5 to 16, as APMLightsComponent lists them");
        assert_eq!(write(&fake, &format!("{}{ENUM_INDEX}", lights1["path"].as_str().unwrap()), r#"{"value":2}"#)["ok"], true);
        assert_eq!((fake.params.borrow()["SERVO9_FUNCTION"], fake.params.borrow()["SERVO6_FUNCTION"]), (0.0, 59.0), "setRCFunction clears the old channel first");
        assert!(rows.iter().any(|r| r["label"] == "Brightness Steps"));
        let board = Fake::new(&named.iter().map(|(n, v)| (n.as_str(), *v)).chain([("JS_LIGHTS_STEPS", 4.0), ("BRD_PWM_COUNT", 2.0)]).collect::<Vec<_>>());
        let rows: Vec<Value> = page(&board, "Lights", false)["sections"].as_array().unwrap().iter().flat_map(|s| s["controls"].as_array().cloned().unwrap_or_default()).collect();
        let lights = rows.iter().find(|r| r["label"] == "Lights 1").unwrap().clone();
        assert_eq!(lights["options"].as_array().unwrap().len(), 7, "8 main outputs plus BRD_PWM_COUNT=2 auxiliaries: Disabled and channels 5 to 10");
        assert_eq!(lights["display"], "Channel 9");
        let far = Fake::new(&named.iter().map(|(n, v)| (n.as_str(), if n == "SERVO9_FUNCTION" { 0.0 } else if n == "SERVO12_FUNCTION" { 59.0 } else { *v })).chain([("JS_LIGHTS_STEPS", 4.0), ("BRD_PWM_COUNT", 2.0)]).collect::<Vec<_>>());
        let rows: Vec<Value> = page(&far, "Lights", false)["sections"].as_array().unwrap().iter().flat_map(|s| s["controls"].as_array().cloned().unwrap_or_default()).collect();
        let lights = rows.iter().find(|r| r["label"] == "Lights 1").unwrap().clone();
        assert_eq!(lights["display"], "Channel 12", "calcLightOutValues scans 5 to 16 even when the list is shorter");
        assert_eq!(write(&far, &format!("{}{ENUM_INDEX}", lights["path"].as_str().unwrap()), r#"{"value":2}"#)["ok"], true);
        assert_eq!((far.params.borrow()["SERVO6_FUNCTION"], far.params.borrow()["SERVO12_FUNCTION"]), (59.0, 0.0), "setRCFunction clears a channel beyond the list too");
        let seven = Fake::new(&named.iter().map(|(n, v)| (n.as_str(), *v)).chain([("JS_LIGHTS_STEPS", 4.0), ("BRD_PWM_COUNT", 7.0)]).collect::<Vec<_>>());
        let rows: Vec<Value> = page(&seven, "Lights", false)["sections"].as_array().unwrap().iter().flat_map(|s| s["controls"].as_array().cloned().unwrap_or_default()).collect();
        assert_eq!(rows.iter().find(|r| r["label"] == "Lights 1").unwrap()["options"].as_array().unwrap().len(), 8, "a BRD_PWM_COUNT of 7 counts as 3");
    }

    #[test]
    fn flight_behavior_switches_a_slider_off_by_making_its_value_negative() {
        let fake = Fake { px4: true, ..Fake::new(&[("SYS_VEHICLE_RESP", 0.9), ("MPC_XY_VEL_ALL", -5.0), ("MPC_Z_VEL_ALL", 2.0), ("NAV_ACC_RAD", 3.0)]) };
        let rows: Vec<Value> = page(&fake, "Flight Behavior", true)["sections"].as_array().unwrap().iter().flat_map(|s| s["controls"].as_array().cloned().unwrap_or_default()).collect();
        assert!(rows.iter().any(|r| r["control"] == "label" && r["label"].as_str().unwrap_or_default().starts_with("Warning: a high responsiveness")), "PX4FlightBehaviorCopter warns above 0.8");
        let horizontal = rows.iter().find(|r| r["control"] == "toggle" && r["name"] == "MPC_XY_VEL_ALL.enable").unwrap().clone();
        assert_eq!(horizontal["label"], "Enable horizontal velocity slider (if enabled, individual velocity limit parameters are automatically set)", "xyVelCheckbox text");
        assert_eq!(horizontal["value"], false, "a negative MPC_XY_VEL_ALL is the slider switched off");
        let gated = rows.iter().find(|r| r["control"] != "toggle" && r["label"] == "Horizontal velocity (m/s)").unwrap();
        assert_eq!(gated["disabledReason"], "Turn on the switch above to edit");
        assert_eq!(gated["description"], "Limit the horizonal velocity (applies to all modes).", "SettingsGroupLayout headingDescription, QGC typo included");
        assert_eq!(write(&fake, horizontal["path"].as_str().unwrap(), r#"{"value":true}"#)["ok"], true);
        assert_eq!(fake.params.borrow()["MPC_XY_VEL_ALL"], 5.0, "switching it on keeps the magnitude");
        assert!(rows.iter().any(|r| r["label"] == "Mission Turning Radius"));
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
        assert_eq!(disabled["keywords"], json!(["disabled batteries"]), "QGC's sidebar matches the disabled heading by its name only");
        let enabled = served["sections"].as_array().unwrap().iter().find(|s| s["title"] != "Disabled Batteries").unwrap();
        assert!(enabled["keywords"].as_array().unwrap().iter().any(|k| k == "capacity"), "{enabled}");
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

    #[test]
    fn airspeed_shows_pitot_rows_only_for_pitot_sensors() {
        let fake = Fake::new(&[("ARSPD_TYPE", 1.0), ("ARSPD_USE", 1.0), ("ARSPD_RATIO", 2.0), ("ARSPD_AUTOCAL", 0.0), ("ARSPD_BUS", 1.0), ("ARSPD_PIN", 15.0), ("ARSPD2_TYPE", 13.0), ("ARSPD2_USE", 0.0), ("ARSPD2_RATIO", 2.0), ("ARSPD_PRIMARY", 0.0), ("AIRSPEED_CRUISE", 18.0), ("ARSPD_WIND_MAX", 0.0)]);
        let served = page(&fake, "Airspeed", false);
        let section = |title: &str| served["sections"].as_array().unwrap().iter().find(|s| s["title"] == title).cloned();
        let labels = |title: &str| section(title).map(|s| s["controls"].as_array().unwrap().iter().map(|c| c["label"].as_str().unwrap().to_string()).collect::<Vec<_>>()).unwrap_or_default();
        assert_eq!(labels("Primary Airspeed Sensor"), ["Sensor type", "Use airspeed", "Airspeed ratio", "Auto calibrate ratio in flight"]);
        assert_eq!(labels("Second Airspeed Sensor"), ["Sensor type", "Use airspeed"], "NMEA is not a pitot sensor");
        assert_eq!(labels("Sensor Settings: Primary Sensor"), ["I2C bus"], "an I2C pitot has neither an analog pin nor a PSI range");
        assert!(section("Multi-Sensor Options").is_some());
        assert!(section("Airspeed Limits").is_some());
        let off = Fake::new(&[("ARSPD_TYPE", 0.0), ("ARSPD_USE", 1.0), ("AIRSPEED_CRUISE", 18.0)]);
        let served = page(&off, "Airspeed", false);
        let titles: Vec<&str> = served["sections"].as_array().unwrap().iter().map(|s| s["title"].as_str().unwrap()).collect();
        assert_eq!(titles, ["Primary Airspeed Sensor"], "a disabled sensor offers only its type");
    }

    #[test]
    fn esc_calibration_arms_once_and_quadplanes_use_their_own_parameters() {
        let fake = Fake::new(&[("MOT_PWM_TYPE", 6.0), ("MOT_PWM_MIN", 1000.0), ("SERVO_DSHOT_ESC", 1.0), ("ESC_CALIBRATION", 0.0)]);
        let served = page(&fake, "ESC", false);
        let section = |served: &Value, title: &str| served["sections"].as_array().unwrap().iter().find(|s| s["title"] == title).cloned().unwrap();
        let labels: Vec<String> = section(&served, "Configuration")["controls"].as_array().unwrap().iter().map(|c| c["label"].as_str().unwrap().to_string()).collect();
        assert_eq!(labels, ["Output type", "Requires vehicle reboot", "Output PWM min", "DShot ESC type"]);
        let calibrate = section(&served, "Calibration")["controls"].as_array().unwrap().iter().find(|c| c["label"] == "Calibrate").cloned().unwrap();
        assert_eq!((calibrate["control"].clone(), calibrate["enabled"].clone()), (json!("button"), json!(true)), "APMESCComponent's Calibrate is a one-shot QGCButton, not a switch");
        let step = |served: &Value| section(served, "Calibration")["controls"].as_array().unwrap().iter().find(|c| c["label"] == "- Connect the battery").cloned().unwrap();
        assert_eq!(step(&served)["enabled"], false, "the steps are greyed until calibration is armed");
        assert_eq!(write(&fake, calibrate["path"].as_str().unwrap(), r#"{"value":true}"#)["ok"], true);
        assert_eq!(fake.params.borrow()["ESC_CALIBRATION"], 3.0);
        let started = page(&fake, "ESC", false);
        assert!(section(&started, "Calibration")["controls"].as_array().unwrap().iter().any(|c| c["label"] == "Now perform these steps:"));
        assert_eq!(step(&started)["enabled"], true);
        let armed = section(&started, "Calibration")["controls"].as_array().unwrap().iter().find(|c| c["label"] == "Calibrate").cloned().unwrap();
        assert_eq!(armed["enabled"], false, "enabled only while the parameter reads 0");
        assert_eq!(write(&fake, armed["path"].as_str().unwrap(), r#"{"value":true}"#)["ok"], false);
        let quad = Fake::new(&[("Q_M_PWM_TYPE", 0.0), ("Q_ESC_CAL", 0.0), ("ESC_CALIBRATION", 0.0)]);
        let served = page(&quad, "ESC", false);
        assert_eq!(section(&served, "Configuration")["controls"][0]["name"], "Q_M_PWM_TYPE");
        assert!(section(&served, "Calibration")["controls"].as_array().unwrap().iter().any(|c| c["name"] == "Q_ESC_CAL"));
    }

    #[test]
    fn px4_radio_switches_hide_flaps_on_a_multirotor() {
        let params = [("RC_MAP_FLAPS", 0.0), ("RC_MAP_AUX1", 6.0), ("RC_MAP_PAY_SW", 0.0)];
        let names = |fake: &Fake| -> Vec<String> { page(fake, RADIO_SWITCHES, true)["sections"].as_array().unwrap().iter().flat_map(|s| s["controls"].as_array().cloned().unwrap_or_default()).filter_map(|c| c["name"].as_str().map(str::to_string)).collect() };
        assert_eq!(names(&Fake { px4: true, ..Fake::new(&params) }), ["RC_MAP_AUX1", "RC_MAP_PAY_SW"]);
        assert_eq!(names(&Fake { px4: true, multi_rotor: false, ..Fake::new(&params) }), ["RC_MAP_FLAPS", "RC_MAP_AUX1", "RC_MAP_PAY_SW"]);
    }

    #[test]
    fn a_sub_gets_its_own_flight_safety_page() {
        let params = [("FS_GCS_ENABLE", 1.0), ("FS_LEAK_ENABLE", 0.0), ("LEAK1_PIN", 27.0), ("FS_TEMP_ENABLE", 1.0), ("FS_TEMP_MAX", 62.0), ("FS_PRESS_ENABLE", 0.0), ("FS_PRESS_MAX", 105000.0), ("FS_EKF_ACTION", 1.0), ("ARMING_CHECK", 1.0)];
        let titles = |fake: &Fake| -> Vec<String> { page(fake, "Flight Safety", false)["sections"].as_array().unwrap().iter().filter_map(|s| s["title"].as_str().map(str::to_string)).collect() };
        let sub = Fake { sub: true, ..Fake::new(&params) };
        assert_eq!(titles(&sub), ["Failsafe Actions", "Arming Checks"]);
        let labels: Vec<String> = page(&sub, "Flight Safety", false)["sections"][0]["controls"].as_array().unwrap().iter().filter_map(|c| c["label"].as_str().map(str::to_string)).collect();
        assert_eq!(labels, ["GCS Heartbeat", "Leak", "Internal Temperature", "Threshold", "Internal Pressure"], "leak pin and pressure threshold hide while disabled; EKF waits for 3.5 parameters");
        assert!(!titles(&Fake::new(&params)).contains(&"Failsafe Actions".to_string()), "a copter keeps the fence page");
    }
}
