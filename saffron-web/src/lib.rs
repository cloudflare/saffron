use chrono::prelude::*;
use js_sys::{Array as JsArray, Date as JsDate, JsString};
use saffron::parse::{CronExpr, English};
use saffron::{Cron, CronTimesIter};
use wasm_bindgen::prelude::*;

fn chrono_to_js_date(date: DateTime<Utc>) -> JsDate {
    let js_millis = JsValue::from_f64(date.timestamp_millis() as f64);
    JsDate::new(&js_millis)
}

/// @private
#[wasm_bindgen]
#[derive(Clone, Debug)]
pub struct WasmCron {
    inner: Cron,
}

#[wasm_bindgen]
impl WasmCron {
    #[wasm_bindgen(constructor)]
    pub fn new(s: &str) -> Result<WasmCron, JsValue> {
        s.parse()
            .map(|inner| Self { inner })
            .map_err(|e| JsString::from(e.to_string()).into())
    }

    #[wasm_bindgen(js_name = parseAndDescribe)]
    pub fn parse_and_describe(s: &str) -> Result<JsArray, JsValue> {
        s.parse()
            .map(move |expr: CronExpr| {
                let description = expr.describe(English::default()).to_string();
                let cron = Self {
                    inner: Cron::new(expr),
                };

                let array = JsArray::new_with_length(2);
                array.set(0, cron.into());
                array.set(1, JsString::from(description).into());
                array
            })
            .map_err(|e| JsString::from(e.to_string()).into())
    }

    pub fn any(&self) -> bool {
        self.inner.any()
    }

    pub fn contains(&self, date: JsDate) -> bool {
        self.inner.contains(date.into())
    }

    #[wasm_bindgen(js_name = nextFrom)]
    pub fn next_from(&self, date: JsDate) -> Option<JsDate> {
        self.inner.next_from(date.into()).map(chrono_to_js_date)
    }

    #[wasm_bindgen(js_name = nextAfter)]
    pub fn next_after(&self, date: JsDate) -> Option<JsDate> {
        self.inner.next_after(date.into()).map(chrono_to_js_date)
    }
}

// Build a iter type that just returns an optional Date on next.
// This doesn't conform to iterator standards, but we can't build
// a conformant iterator with wasm anyway, so let's just export what
// we need to do it fast and build our iterator type in js.

/// @private
#[wasm_bindgen]
pub struct WasmCronTimesIter {
    inner: CronTimesIter,
}

#[wasm_bindgen]
impl WasmCronTimesIter {
    #[wasm_bindgen(js_name = startFrom)]
    pub fn start_from(cron: &WasmCron, date: JsDate) -> Self {
        Self {
            inner: cron.inner.clone().iter_from(date.into()),
        }
    }

    #[wasm_bindgen(js_name = startAfter)]
    pub fn start_after(cron: &WasmCron, date: JsDate) -> Self {
        Self {
            inner: cron.inner.clone().iter_after(date.into()),
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Option<JsDate> {
        self.inner.next().map(chrono_to_js_date)
    }
}
