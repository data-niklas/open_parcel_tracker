use crate::{Carrier, CarrierParcel, CarrierParcelEvent, TrackingError};
use chrono::{DateTime, Utc};
use ehttp::{fetch_async, Request};
use futures::future::join_all;
use icu_locid::subtags::Language;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct SendungsEvent {
    datum: DateTime<Utc>,
    status: String,
}

#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
struct Sendungsverlauf {
    events: Vec<SendungsEvent>,
    kurzStatus: String,
}

#[derive(Debug, Deserialize)]
struct Sendungsdetails {
    sendungsverlauf: Sendungsverlauf,
    zielland: String,
    quelle: String,
}

#[derive(Debug, Deserialize)]
struct Sendungsinfo {
    sendungsname: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Sendung {
    sendungsdetails: Sendungsdetails,
    sendungsinfo: Sendungsinfo,
}

#[derive(Debug, Deserialize)]
struct ResponseBody {
    sendungen: Vec<Sendung>,
}

pub async fn track_single(
    parcel_id: &str,
    locale: &Language,
) -> Result<Option<CarrierParcel>, TrackingError> {
    let url = format!(
        "https://www.dhl.de/int-verfolgen/data/search?piececode={}&language={}",
        parcel_id, locale
    );
    let mut request = Request::get(url);
    request.headers.insert("Accept", "application/json");
    request.headers.insert("Accept-Language", locale);
    request.headers.insert("Content-Type", "application/json");
    request.headers.insert("Host", "www.dhl.de");
    request.headers.insert("Cache-Control", "keep-alive");

    let response = match fetch_async(request).await {
        Ok(response) => response,
        Err(e) => return Err(TrackingError::RequestError(e)),
    };
    let deserialized_response: Option<ResponseBody> = response.json().ok();
    if deserialized_response.is_none() {
        return Ok(None);
    }
    let body = deserialized_response.unwrap();
    let events = body.sendungen[0]
        .sendungsdetails
        .sendungsverlauf
        .events
        .iter()
        .map(|event| CarrierParcelEvent {
            datetime: event.datum,
            description: event.status.clone(),
            region: None,
        })
        .collect();
    let start_region = None;
    let sendungsdetails = &body.sendungen[0].sendungsdetails;
    let end_region = sendungsdetails.zielland.clone();
    let status = sendungsdetails.sendungsverlauf.kurzStatus.clone();
    let product = Some(sendungsdetails.quelle.clone());
    let name = body.sendungen[0].sendungsinfo.sendungsname.clone();

    Ok(Some(CarrierParcel {
        id: parcel_id.to_owned(),
        events,
        start_region,
        end_region,
        status,
        carrier: Carrier::DHL,
        product,
        name,
    }))
}

pub async fn track(
    parcels: Vec<&str>,
    locale: Language,
) -> Result<Vec<Option<CarrierParcel>>, TrackingError> {
    let mut results = Vec::with_capacity(parcels.len());
    for parcel in parcels {
        results.push(track_single(parcel, &locale));
    }
    join_all(results)
        .await
        .into_iter()
        .collect::<Result<Vec<Option<CarrierParcel>>, TrackingError>>()
}
