use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
    sync::{Arc, Mutex},
};

use chrono::{DateTime, Utc};
use icu_locid::LanguageIdentifier;
use open_parcel_tracker::{track_parcels, Carrier, Parcel, ParcelEvent, TrackingError};
use rocket::{
    http::Status,
    outcome::IntoOutcome,
    request::{FromRequest, Outcome},
    serde::json::Json,
    Request, State,
};
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;
#[macro_use]
extern crate rocket;

#[allow(non_snake_case)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SyncParcel {
    pub id: String,
    pub start_region: Option<String>,
    pub end_region: String,
    pub status: String,
    pub product: Option<String>,
    pub events: Vec<ParcelEvent>,
    pub carriers: Vec<Carrier>,
    pub name: Option<String>,
    // Custom JS attributes
    pub archived: bool,
    pub addTime: DateTime<Utc>,
}

impl Hash for SyncParcel {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

#[derive(Deserialize)]
struct TrackRequest {
    parcels: Vec<(String, Vec<Carrier>)>,
    language: LanguageIdentifier,
}

#[get("/")]
fn default() -> rocket::response::content::RawHtml<&'static [u8]> {
    let body = include_bytes!("../assets/index.html");
    rocket::response::content::RawHtml(body)
}

#[get("/index.html")]
fn index_html() -> rocket::response::content::RawHtml<&'static [u8]> {
    let body = include_bytes!("../assets/index.html");
    rocket::response::content::RawHtml(body)
}

#[get("/index.js")]
fn index_js() -> rocket::response::content::RawJavaScript<&'static [u8]> {
    let body = include_bytes!("../assets/index.js");
    rocket::response::content::RawJavaScript(body)
}

#[get("/index.css")]
fn index_css() -> rocket::response::content::RawCss<&'static [u8]> {
    let body = include_bytes!("../assets/index.css");
    rocket::response::content::RawCss(body)
}

#[get("/carriers")]
fn carriers() -> Json<Vec<Carrier>> {
    let carriers = Carrier::iter().collect::<Vec<Carrier>>();
    carriers.into()
}

#[post("/track", data = "<json>")]
async fn track(json: Json<TrackRequest>) -> Json<Result<Vec<Option<Parcel>>, TrackingError>> {
    let json: TrackRequest = json.into_inner();
    let parcels = track_parcels(&json.parcels, json.language.language).await;
    parcels.into()
}

// -------------- SYNC -----------------
struct XTokenSubject<'h>(&'h str);
#[rocket::async_trait]
impl<'r> FromRequest<'r> for XTokenSubject<'r> {
    type Error = Status;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let request_headers = request.headers();
        let maybe_x_token_subject = request_headers
            .get_one("X-Token-Subject")
            .map(XTokenSubject)
            .ok_or(Status::BadRequest);
        maybe_x_token_subject.or_forward(Status::InternalServerError)
    }
}

pub type SyncStorage = HashMap<String, HashSet<SyncParcel>>;
pub type SyncStorageWrapper = Arc<Mutex<SyncStorage>>;

#[post("/sync", data = "<json>")]
async fn sync(
    json: Json<Vec<SyncParcel>>,
    user_name: XTokenSubject<'_>,
    storage_state: &State<SyncStorageWrapper>,
) -> Json<Vec<SyncParcel>> {
    let mut storage = storage_state.lock().unwrap();
    if !storage.contains_key(user_name.0) {
        storage.insert(user_name.0.to_string(), HashSet::new());
    }
    let user_data = storage.get_mut(user_name.0).unwrap();
    for parcel in json.iter() {
        if user_data.contains(parcel) {
            let other_parcel = user_data.get(parcel).unwrap();
            let other_parcel_fresher = other_parcel.events.len() > parcel.events.len();
            if !other_parcel_fresher {
                user_data.insert(parcel.clone());
            }
        } else {
            user_data.insert(parcel.clone());
        }
    }
    let mut result = Vec::new();
    for parcel in user_data.iter() {
        result.push(parcel.clone());
    }
    Json(result)
}

struct SupportsSync(bool);
#[get("/has_sync")]
fn has_sync(has_sync: &State<SupportsSync>) -> Json<bool> {
    Json(has_sync.0)
}

#[rocket::main]
async fn main() {
    let enable_sync = std::env::var("ENABLE_SYNC").is_ok();
    let mut routes = routes![default, index_html, index_js, index_css, carriers, track, has_sync];
    if enable_sync {
        routes.extend(routes![sync]);
    }
    let mut rocket_pre_build = rocket::build().manage(SupportsSync(enable_sync));
    if enable_sync {
        rocket_pre_build = rocket_pre_build.manage(Arc::new(Mutex::new(SyncStorage::new())));
    }
    let _ = rocket_pre_build.mount("/", routes).launch().await;
}
