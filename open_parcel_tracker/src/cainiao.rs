use crate::{Carrier, CarrierParcel, CarrierParcelEvent, TrackingError};
use chrono::{TimeZone, Utc};
use ehttp::{fetch_async, Request};
use icu_locid::Locale;
use serde::Deserialize;

#[derive(Deserialize)]
#[allow(non_snake_case)]
struct Detail {
    time: i64,
    desc: String,
    descTitle: String,
}

#[derive(Deserialize)]
#[allow(non_snake_case)]
struct Module {
    originCountry: String,
    destCountry: String,
    statusDesc: String,
    detailList: Vec<Detail>,
}

#[derive(Deserialize)]
struct ResponseBody {
    module: Vec<Module>,
}

pub async fn track(
    parcels: Vec<&str>,
    locale: Locale,
) -> Result<Vec<Option<CarrierParcel>>, TrackingError> {
    let joined_parcel_ids = parcels.join(",");
    let url = format!(
        "https://global.cainiao.com/global/detail.json?mailNos={}&lang={}&language={}",
        joined_parcel_ids, locale.id.language, locale.id.language
    );
    let mut request = Request::get(url);
    request.headers.insert("Accept", "application/json");
    request
        .headers
        .insert("Accept-Language", locale.id.language);
    request.headers.insert("Content-Type", "application/json");

    let response = match fetch_async(request).await {
        Ok(response) => response,
        Err(e) => return Err(TrackingError::RequestError(e)),
    };
    let deserialized_response: Option<ResponseBody> = response.json().ok();
    if deserialized_response.is_none() {
        return Ok(vec![None; parcels.len()]);
    }
    let body = deserialized_response.unwrap();
    Ok(body
        .module
        .into_iter()
        .zip(parcels)
        .map(|(module, id)| {
            let events = module
                .detailList
                .iter()
                .map(|detail| CarrierParcelEvent {
                    datetime: Utc.timestamp_opt(detail.time / 1000, 0).single().unwrap(),
                    description: format!("{}: {}", detail.descTitle, detail.desc),
                    region: None,
                })
                .collect();
            let start_region = Some(module.originCountry.clone());
            let end_region = module.destCountry.clone();
            let status = module.statusDesc.clone();
            Some(CarrierParcel {
                id: id.to_owned(),
                events,
                start_region,
                end_region,
                status,
                carrier: Carrier::Cainiao,
                product: None,
                name: None,
            })
        })
        .collect())
}

