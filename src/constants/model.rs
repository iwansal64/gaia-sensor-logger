use mongodb::bson::DateTime;
use serde::{Serialize, Deserialize};
use sqlx::FromRow;
use chrono::{NaiveDateTime};

#[derive(Debug, Serialize, Deserialize)]
pub struct Metadata {
    pub device_id: String
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SensorData {
    pub metadata: Metadata,
    pub timestamp: DateTime,
    pub data: f32
}

#[derive(Debug, FromRow)]
pub struct Device {
    pub id: String,
    pub created_at: NaiveDateTime,
    pub access_token: Option<String>,
    pub device_name: String,
    pub description: Option<String>,
    pub status: bool,
    pub last_online: Option<NaiveDateTime>,
}
