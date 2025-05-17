use crate::{Carrier, CarrierParcel, CarrierParcelEvent, TrackingError};
use chrono::{DateTime, Utc};
use ehttp::{fetch_async, Request};
use futures::future::join_all;
use icu_locid::subtags::Language;
use serde::{Deserialize, Serialize};
use serde_json;

#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
struct Track {
    tkDate: DateTime<Utc>,
    tkTranslatedDesc: String,
}

#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
struct ParcelData {
    ctStartName: String,
    ctEndName: String,
    tracks: Option<Vec<Track>>,
}

#[derive(Debug, Deserialize)]
struct ResponseBody {
    data: Vec<ParcelData>,
}
#[derive(Serialize)]
#[allow(non_snake_case)]
struct RequestParams {
    language: String,
    queryCodes: Vec<String>,
    translateLanguage: String,
}
pub async fn track_single(
    parcel_id: &str,
    locale: &Language,
) -> Result<Option<CarrierParcel>, TrackingError> {
    let url = "https://track.4px.com/track/v2/front/listTrackV3";
    let request_params = RequestParams {
        language: locale.as_str().to_owned(),
        queryCodes: vec![parcel_id.to_owned()],
        translateLanguage: locale.as_str().to_string(),
    };
    let mut request = Request::post(url, serde_json::to_vec(&request_params).unwrap());
    request.headers.insert("Accept", "application/json");
    request.headers.insert("Accept-Language", locale);
    request.headers.insert("Content-Type", "application/json");
    request.headers.insert("Host", "track.4px.com");
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

    let tracks = match &body.data[0].tracks {
        Some(tracks) => tracks,
        None => return Ok(None),
    };
    let events: Vec<CarrierParcelEvent> = tracks
        .iter()
        .map(|track| CarrierParcelEvent {
            datetime: track.tkDate,
            description: track.tkTranslatedDesc.clone(),
            region: None,
        })
        .collect();
    let status = events[0].description.clone();
    let start_region = Some(body.data[0].ctStartName.clone());
    let end_region = body.data[0].ctEndName.clone();
    Ok(Some(CarrierParcel {
        id: parcel_id.to_owned(),
        events,
        start_region,
        end_region,
        status,
        carrier: Carrier::FourPX,
        product: None,
        name: None,
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
