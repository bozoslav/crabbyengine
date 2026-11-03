//! Field paths, `where` conditions and statistics over reflected components,
//! for the control endpoint's `entities.list` and `entities.stats`.

use std::cmp::Ordering;
use std::collections::HashMap;

use serde_json::{Value, json};

/// How many distinct values `entities.stats` counts per field before it
/// lumps the rest together.
const MAX_DISTINCT: usize = 32;

/// `Health.current`, `Transform2D.position.0`: a component, then fields
/// (array elements by number). Just `Team` means the whole component.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FieldPath {
    pub(crate) component: String,
    fields: Vec<String>,
    text: String,
}

impl FieldPath {
    pub(crate) fn parse(text: &str) -> Result<Self, String> {
        let mut parts = text.trim().split('.');
        let component = parts.next().unwrap_or_default().to_string();
        let fields: Vec<String> = parts.map(str::to_string).collect();
        if component.is_empty() || fields.iter().any(String::is_empty) {
            return Err(format!("`{text}` is not a field path (like `Health.current`)"));
        }
        Ok(Self {
            component,
            fields,
            text: text.trim().to_string(),
        })
    }

    /// The value at this path in a component's JSON.
    pub(crate) fn get<'a>(&self, component: &'a Value) -> Option<&'a Value> {
        self.fields.iter().try_fold(component, |v, f| match v {
            Value::Object(o) => o.get(f),
            Value::Array(a) => a.get(f.parse::<usize>().ok()?),
            _ => None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Op {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

/// `Health.current < 5`, `Team == "Red"`, `Unit.alive == true`.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Condition {
    pub(crate) path: FieldPath,
    op: Op,
    value: Value,
}

impl Condition {
    pub(crate) fn parse(text: &str) -> Result<Self, String> {
        // Two-character operators first, so `<=` isn't read as `<`.
        const OPS: [(&str, Op); 6] = [
            ("==", Op::Eq),
            ("!=", Op::Ne),
            ("<=", Op::Le),
            (">=", Op::Ge),
            ("<", Op::Lt),
            (">", Op::Gt),
        ];
        let bad = || format!("`{text}` is not a condition (like `Health.current < 5` or `Team == \"Red\"`)");
        let (at, sym, op) = OPS
            .iter()
            .filter_map(|&(sym, op)| text.find(sym).map(|at| (at, sym, op)))
            .min_by_key(|&(at, sym, _)| (at, std::cmp::Reverse(sym.len())))
            .ok_or_else(bad)?;
        let path = FieldPath::parse(&text[..at]).map_err(|_| bad())?;
        let raw = text[at + sym.len()..].trim();
        if raw.is_empty() {
            return Err(bad());
        }
        // A JSON literal, or a bare word meaning a string: `Team == Red`.
        let value = serde_json::from_str(raw).unwrap_or_else(|_| Value::String(raw.to_string()));
        Ok(Self { path, op, value })
    }

    /// Whether a component's JSON passes. Missing fields never pass.
    pub(crate) fn holds(&self, component: &Value) -> bool {
        let Some(v) = self.path.get(component) else {
            return false;
        };
        let order = match (v, &self.value) {
            (Value::Number(a), Value::Number(b)) => a.as_f64().partial_cmp(&b.as_f64()),
            (Value::String(a), Value::String(b)) => Some(a.cmp(b)),
            (a, b) => (a == b).then_some(Ordering::Equal),
        };
        match self.op {
            Op::Eq => order == Some(Ordering::Equal),
            Op::Ne => order != Some(Ordering::Equal),
            Op::Lt => order == Some(Ordering::Less),
            Op::Le => matches!(order, Some(Ordering::Less | Ordering::Equal)),
            Op::Gt => order == Some(Ordering::Greater),
            Op::Ge => matches!(order, Some(Ordering::Greater | Ordering::Equal)),
        }
    }
}

/// Running numbers for one field: plain numbers, or arrays of numbers
/// (vectors) element by element.
#[derive(Default)]
pub(crate) struct FieldStats {
    count: u64,
    min: Vec<f64>,
    max: Vec<f64>,
    sum: Vec<f64>,
    /// Non-numeric values (strings, enum variants, bools) by their JSON text.
    values: HashMap<String, u64>,
    other_values: u64,
}

impl FieldStats {
    pub(crate) fn add(&mut self, v: &Value) {
        self.count += 1;
        let numbers: Option<Vec<f64>> = match v {
            Value::Number(n) => n.as_f64().map(|x| vec![x]),
            Value::Array(a) if !a.is_empty() => a.iter().map(Value::as_f64).collect(),
            _ => None,
        };
        match numbers {
            Some(xs) if self.sum.is_empty() || self.sum.len() == xs.len() => {
                if self.sum.is_empty() {
                    self.min = vec![f64::INFINITY; xs.len()];
                    self.max = vec![f64::NEG_INFINITY; xs.len()];
                    self.sum = vec![0.0; xs.len()];
                }
                for (i, x) in xs.into_iter().enumerate() {
                    self.min[i] = self.min[i].min(x);
                    self.max[i] = self.max[i].max(x);
                    self.sum[i] += x;
                }
            }
            _ => {
                let key = match v {
                    Value::String(s) => s.clone(),
                    v => v.to_string(),
                };
                if self.values.len() < MAX_DISTINCT || self.values.contains_key(&key) {
                    *self.values.entry(key).or_default() += 1;
                } else {
                    self.other_values += 1;
                }
            }
        }
    }

    pub(crate) fn to_json(&self) -> Value {
        let mut out = json!({ "count": self.count });
        if !self.sum.is_empty() {
            let numbers = self.count - self.values.values().sum::<u64>() - self.other_values;
            let one = |xs: &[f64]| if xs.len() == 1 { json!(xs[0]) } else { json!(xs) };
            let mean: Vec<f64> = self.sum.iter().map(|s| s / numbers as f64).collect();
            out["min"] = one(&self.min);
            out["max"] = one(&self.max);
            out["mean"] = one(&mean);
        }
        if !self.values.is_empty() {
            let mut values: Vec<(&String, &u64)> = self.values.iter().collect();
            values.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
            out["values"] = values.into_iter().map(|(k, n)| (k.clone(), json!(n))).collect();
            if self.other_values > 0 {
                out["other_values"] = json!(self.other_values);
            }
        }
        out
    }
}

/// Parses a list param of strings with `parse`.
pub(crate) fn list<T>(params: &Value, name: &str, parse: fn(&str) -> Result<T, String>) -> Result<Vec<T>, String> {
    match params.get(name) {
        None | Some(Value::Null) => Ok(Vec::new()),
        Some(Value::String(s)) => Ok(vec![parse(s)?]),
        Some(Value::Array(items)) => items
            .iter()
            .map(|i| {
                i.as_str()
                    .ok_or_else(|| format!("`{name}` must be a list of strings"))
                    .and_then(parse)
            })
            .collect(),
        Some(_) => Err(format!("`{name}` must be a list of strings")),
    }
}

impl FieldPath {
    pub(crate) fn text(&self) -> &str {
        &self.text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conditions_parse_and_compare() {
        let unit = json!({ "current": 3, "team": "Red", "at": [1.0, 2.0] });
        let holds = |c: &str| Condition::parse(c).unwrap().holds(&unit);
        assert!(holds("Health.current < 5"));
        assert!(holds("Health.current <= 3"));
        assert!(!holds("Health.current > 3"));
        assert!(holds("Health.current != 4"));
        assert!(holds("Health.team == \"Red\""));
        assert!(holds("Health.team == Red"));
        assert!(holds("Health.at.1 >= 2"));
        assert!(!holds("Health.missing == 1"), "missing fields never pass");
        assert!(Condition::parse("Health.current").is_err());
        assert!(Condition::parse("< 3").is_err());
        let whole = Condition::parse("Team == Blue").unwrap();
        assert!(whole.holds(&json!("Blue")));
    }

    #[test]
    fn stats_cover_numbers_vectors_and_names() {
        let mut n = FieldStats::default();
        for x in [1, 5, 3] {
            n.add(&json!(x));
        }
        assert_eq!(n.to_json(), json!({ "count": 3, "min": 1.0, "max": 5.0, "mean": 3.0 }));
        let mut v = FieldStats::default();
        v.add(&json!([0.0, 10.0]));
        v.add(&json!([2.0, 20.0]));
        assert_eq!(v.to_json()["mean"], json!([1.0, 15.0]));
        let mut t = FieldStats::default();
        for team in ["Red", "Blue", "Red"] {
            t.add(&json!(team));
        }
        assert_eq!(t.to_json(), json!({ "count": 3, "values": { "Red": 2, "Blue": 1 } }));
    }

    #[test]
    fn distinct_values_are_capped() {
        let mut t = FieldStats::default();
        for i in 0..100 {
            t.add(&json!(format!("v{i}")));
        }
        let j = t.to_json();
        assert_eq!(j["values"].as_object().unwrap().len(), MAX_DISTINCT);
        assert_eq!(j["other_values"], 100 - MAX_DISTINCT as u64);
    }
}
