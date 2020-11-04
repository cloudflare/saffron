use cfron::{parse::CronExpr, Cron};
use chrono::{DateTime, Utc};
use js_sys::{Array as JsArray, Date, JsString};
use wasm_bindgen::prelude::*;

use std::collections::HashMap;

#[wasm_bindgen]
extern "C" {
    static env: String;
}

fn set_panic_hook() {
    if *env == "dev" {
        console_error_panic_hook::set_once();
    }
}

#[wasm_bindgen]
#[derive(Clone, Debug)]
pub struct Description {
    text: String,
    est_future_executions: Vec<DateTime<Utc>>,
}

#[wasm_bindgen]
impl Description {
    #[wasm_bindgen(getter)]
    pub fn text(&self) -> JsString {
        JsString::from(self.text.as_str())
    }

    #[wasm_bindgen(getter)]
    pub fn est_future_executions(&self) -> JsArray {
        self.est_future_executions
            .iter()
            .map(|dt| {
                let ms = dt.timestamp_millis();
                let value = JsValue::from_f64(ms as f64);
                Date::new(&value)
            })
            .collect()
    }
}

#[wasm_bindgen]
#[derive(Clone, Debug, Default)]
pub struct DescriptionResult {
    description: Option<Description>,
    errors: Option<Vec<String>>,
}

#[wasm_bindgen]
impl DescriptionResult {
    #[wasm_bindgen(getter)]
    pub fn errors(&self) -> Option<JsArray> {
        self.errors
            .as_ref()
            .map(|lst| lst.iter().map(JsValue::from).collect())
    }

    #[wasm_bindgen(getter)]
    pub fn description(&self) -> JsValue {
        JsValue::from(self.description.clone())
    }
}

/// Describes a given cron string. Used for live cron previews on the dash if wasm isn't available.
#[wasm_bindgen]
pub fn describe(cron: &str) -> DescriptionResult {
    set_panic_hook();

    match cron.parse::<CronExpr>() {
        Ok(expr) => {
            let compiled = Cron::new(expr).expect("Parsed valid cron expression");
            let est_future_executions = compiled.iter_from(Utc::now()).take(5).collect();

            DescriptionResult {
                description: Some(Description {
                    text: "not implemented".to_owned(),
                    est_future_executions,
                }),
                ..DescriptionResult::default()
            }
        }
        Err(err) => DescriptionResult {
            errors: Some(vec![format!("{}", err)]),
            ..DescriptionResult::default()
        },
    }
}

#[wasm_bindgen]
#[derive(Clone, Debug)]
pub struct ValidationResult {
    errors: Option<Vec<String>>,
}

#[wasm_bindgen]
impl ValidationResult {
    #[wasm_bindgen]
    pub fn errors(&self) -> Option<JsArray> {
        self.errors
            .as_ref()
            .map(|lst| lst.iter().map(JsValue::from).collect())
    }
}

/// Validates multiple strings. This checks for duplicate expressions and makes sure all expressions
/// can properly compile. The Cloudflare API will perform this check as well.
#[wasm_bindgen]
pub fn validate(crons: JsArray) -> ValidationResult {
    set_panic_hook();

    let len = crons.length();
    let mut map = HashMap::with_capacity(len as usize);
    for i in 0..len {
        let string = match crons.get(i).as_string() {
            Some(string) => string,
            None => {
                return ValidationResult {
                    errors: Some(vec![format!("Element '{}' is not a string", i)]),
                }
            }
        };

        let cron: Cron = match string.parse() {
            Ok(cron) => cron,
            Err(err) => {
                return ValidationResult {
                    errors: Some(vec![format!(
                        "Failed to parse expression at index '{}': {}",
                        i, err
                    )]),
                }
            }
        };

        if let Some(old_str) = map.insert(cron, string.clone()) {
            return ValidationResult {
                errors: Some(vec![format!(
                    "Expression '{}' already exists in the form of '{}'",
                    string, old_str
                )]),
            };
        }
    }

    ValidationResult { errors: None }
}

#[wasm_bindgen]
#[derive(Clone, Debug, Default)]
pub struct NextResult {
    next: Option<DateTime<Utc>>,
    errors: Option<Vec<String>>,
}

#[wasm_bindgen]
impl NextResult {
    #[wasm_bindgen(getter)]
    pub fn errors(&self) -> Option<JsArray> {
        self.errors
            .as_ref()
            .map(|lst| lst.iter().map(JsValue::from).collect())
    }

    #[wasm_bindgen(getter)]
    pub fn next(&self) -> Option<Date> {
        self.next.map(|dt| {
            let ms = dt.timestamp_millis();
            let value = JsValue::from_f64(ms as f64);
            Date::new(&value)
        })
    }
}

#[wasm_bindgen]
pub fn next(cron: &str) -> NextResult {
    set_panic_hook();

    match cron.parse::<Cron>() {
        Ok(expr) => NextResult {
            next: expr.next_from(Utc::now()),
            ..NextResult::default()
        },
        Err(err) => NextResult {
            errors: Some(vec![format!("{}", err)]),
            ..NextResult::default()
        },
    }
}

#[wasm_bindgen]
pub fn next_of_many(crons: JsArray) -> NextResult {
    set_panic_hook();

    let now = Utc::now();
    let mut next = None;
    for (i, value) in (0..crons.length()).map(|i| (i, crons.get(i))) {
        if let Some(string) = value.as_string() {
            match string.parse::<Cron>() {
                Ok(expr) => {
                    if let Some(expr_next) = expr.next_from(now) {
                        match &mut next {
                            Some(next) => *next = std::cmp::min(*next, expr_next),
                            next @ None => *next = Some(expr_next),
                        }
                    }
                }
                Err(err) => {
                    return NextResult {
                        errors: Some(vec![format!("{}", err)]),
                        ..NextResult::default()
                    }
                }
            }
        } else {
            return NextResult {
                errors: Some(vec![format!("Element '{}' is not a string", i)]),
                ..NextResult::default()
            };
        }
    }

    NextResult {
        next,
        ..NextResult::default()
    }
}
