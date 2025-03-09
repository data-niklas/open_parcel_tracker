use icu_locid::LanguageIdentifier;
use open_parcel_tracker::{track_parcels as core_track_parcels, Carrier, TrackingError};
use serde_wasm_bindgen::{from_value, to_value};
use wasm_bindgen::prelude::*;

fn map_error(e: TrackingError) -> JsValue {
    match to_value(&e) {
        Ok(js_value) => js_value,
        Err(e) => e.into(),
    }
}

#[wasm_bindgen]
pub async fn track_parcels(js_parcel: JsValue, js_locale: JsValue) -> Result<JsValue, JsValue> {
    let parcels: Vec<(String, Vec<Carrier>)> = from_value(js_parcel)?;
    let locale: LanguageIdentifier = from_value(js_locale)?;
    let parcel_result = core_track_parcels(&parcels[..], locale.language)
        .await
        .map_err(map_error)?;
    let js_result = to_value(&parcel_result)?;
    Ok(js_result)
}
