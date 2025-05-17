use chrono::{DateTime, Utc};
use futures::future::join_all;
use icu_locid::subtags::Language;
use serde::{Deserialize, Serialize};
use strum::{EnumCount, IntoEnumIterator};
use strum_macros::{EnumCount, EnumIter};

pub mod cainiao;
pub mod dhl;
pub mod four_px;

// TODO: estimation, receiver postcode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CarrierParcel {
    pub id: String,
    pub start_region: Option<String>,
    pub end_region: String,
    pub status: String,
    pub product: Option<String>,
    pub events: Vec<CarrierParcelEvent>,
    pub carrier: Carrier,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CarrierParcelEvent {
    pub datetime: DateTime<Utc>,
    pub region: Option<String>,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Parcel {
    pub id: String,
    pub start_region: Option<String>,
    pub end_region: String,
    pub status: String,
    pub product: Option<String>,
    pub events: Vec<ParcelEvent>,
    pub carriers: Vec<Carrier>,
    pub name: Option<String>,
}

impl std::hash::Hash for Parcel {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl TryFrom<Vec<CarrierParcel>> for Parcel {
    type Error = ();

    fn try_from(value: Vec<CarrierParcel>) -> Result<Self, Self::Error> {
        if value.is_empty() {
            return Err(());
        }
        let carriers = value
            .iter()
            .map(|carrier_parcel| carrier_parcel.carrier)
            .collect();
        let mut events: Vec<ParcelEvent> = value
            .iter()
            .flat_map(|carrier_parcel| {
                carrier_parcel.events.iter().map(|event| ParcelEvent {
                    datetime: event.datetime,
                    region: event.region.clone(),
                    description: event.description.clone(),
                    carrier: carrier_parcel.carrier,
                })
            })
            .collect();
        events.sort_by_key(|event| event.datetime);
        events.reverse();
        let carrier_parcel = value.first().unwrap();
        let name = value
            .iter()
            .filter(|carrier_parcel| carrier_parcel.name.is_some())
            .next()
            .map(|carrier_parcel| carrier_parcel.name.clone().unwrap());
        let product = value
            .iter()
            .filter(|carrier_parcel| carrier_parcel.product.is_some())
            .next()
            .map(|carrier_parcel| carrier_parcel.product.clone().unwrap());
        Ok(Self {
            id: carrier_parcel.id.clone(),
            start_region: carrier_parcel.start_region.clone(),
            end_region: carrier_parcel.end_region.clone(),
            status: carrier_parcel.status.clone(),
            product,
            name,
            events,
            carriers,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ParcelEvent {
    pub datetime: DateTime<Utc>,
    pub region: Option<String>,
    pub description: String,
    pub carrier: Carrier,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, EnumCount, EnumIter, PartialEq, Eq)]
#[cfg_attr(feature = "cli", derive(clap::ValueEnum))]
pub enum Carrier {
    Cainiao,
    DHL,
    FourPX,
}

impl Carrier {
    pub fn index(&self) -> usize {
        *self as usize
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrackingError {
    RequestError(String),
}

impl std::fmt::Display for TrackingError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            TrackingError::RequestError(e) => write!(f, "Request error: {}", e),
        }
    }
}

impl std::error::Error for TrackingError {}

#[trait_variant::make(SendableCarrierSrevice: Send)]
pub trait CarrierService {
    async fn track(
        self,
        parcels: Vec<&str>,
        locale: Language,
    ) -> Result<Vec<Option<CarrierParcel>>, TrackingError>;
}

impl CarrierService for Carrier {
    async fn track(
        self,
        parcels: Vec<&str>,
        locale: Language,
    ) -> Result<Vec<Option<CarrierParcel>>, TrackingError> {
        match self {
            Carrier::Cainiao => cainiao::track(parcels, locale).await,
            Carrier::DHL => dhl::track(parcels, locale).await,
            Carrier::FourPX => four_px::track(parcels, locale).await,
        }
    }
}

pub async fn track_parcels(
    parcels: &[(String, Vec<Carrier>)],
    locale: Language,
) -> Result<Vec<Option<Parcel>>, TrackingError> {
    let mut parcels_per_carrier: [Vec<&str>; Carrier::COUNT] = Default::default();
    for (parcel, carriers) in parcels.iter() {
        for carrier in carriers.iter() {
            parcels_per_carrier[carrier.index()].push(parcel);
        }
    }
    let mut results = Vec::with_capacity(Carrier::COUNT);
    for (carrier, parcels) in Carrier::iter().zip(parcels_per_carrier.into_iter()) {
        results.push(carrier.track(parcels, locale.clone()));
    }
    let mut resolved_results: Vec<Vec<Option<CarrierParcel>>> = join_all(results)
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;
    let collected_results = parcels
        .iter()
        .map(|(_parcel_id, carriers)| {
            carriers
                .into_iter()
                .filter_map(|carrier| resolved_results[carrier.index()].remove(0))
                .collect::<Vec<CarrierParcel>>()
                .try_into()
                .ok()
        })
        .collect::<Vec<Option<Parcel>>>();
    Ok(collected_results)
}
