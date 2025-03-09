use icu_locid::LanguageIdentifier;
use open_parcel_tracker::{track_parcels, Carrier, Parcel, TrackingError};
use rocket::serde::json::Json;
use serde::Deserialize;
use strum::IntoEnumIterator;
#[macro_use]
extern crate rocket;

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
    let json = json.into_inner();
    let parcels = track_parcels(&json.parcels, json.language.language).await;
    parcels.into()
}

#[launch]
fn rocket() -> _ {
    rocket::build().mount(
        "/",
        routes![default, index_html, index_js, index_css, carriers, track],
    )
}
