use chrono::prelude::*;
use js_sys::Date as JsDate;
use wasm_bindgen::prelude::*;

fn chrono_to_js_date(date: DateTime<Utc>) -> JsDate {
    let js_millis = JsValue::from_f64(date.timestamp_millis() as f64);
    JsDate::new(&js_millis)
}

/// @private
#[wasm_bindgen]
#[derive(Clone, Debug)]
pub struct WasmCron {
    inner: saffron::Cron,
}

#[wasm_bindgen]
impl WasmCron {
    #[wasm_bindgen(constructor)]
    pub fn new(s: &str) -> Result<WasmCron, JsValue> {
        match s.parse() {
            Ok(inner) => Ok(Self { inner }),
            Err(err) => Err(JsValue::from_str(&err.to_string())),
        }
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
    inner: saffron::CronTimesIter,
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
