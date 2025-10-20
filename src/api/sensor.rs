use chrono::{Duration, Utc};
use rocket::{futures::TryStreamExt, get, http, serde::json::Json, State};
use serde::{Deserialize, Serialize};
use mongodb::bson::{doc, DateTime};

use crate::constants::{model::SensorData, types::CollectionsData};

#[derive(Serialize)]
pub struct GetSensorResponse {
      pub data: Vec<SensorData>
}


#[derive(Deserialize)]
pub struct GetSensorRequest {
      pub device_id: String,
      pub topic: String
}

#[get("/get", format="json", data="<body>")]
pub async fn get_sensor(body: Json<GetSensorRequest>, collections: &State<CollectionsData>) -> Result<Json<GetSensorResponse>, http::Status> {
      // Lock the collections
      let locked_collections = collections.read().await;
      
      // Get the sensor collection
      let sensor_collection = locked_collections.get(&body.topic);

      let sensor_collection = match sensor_collection {
            Some(data) => data,
            None => {
                  return Err(http::Status::NotFound);
            }
      };


      // Get sensor data
      let start = Utc::now() - Duration::days(1);
      let filter = doc! {
            "metadata.device_id": body.device_id.clone(),
            "timestamp": {
                  "$gte": DateTime::from_millis(start.timestamp_millis())
            }
      };

      let sensors_data = sensor_collection.find(filter).await;

      let mut sensors_data = match sensors_data {
            Ok(data) => data,
            Err(err) => {
                  println!("[API] There's an error when trying to get sensor data from MongoDB. Error: {}", err.to_string());
                  return Err(http::Status::InternalServerError);
            }
      };

      let mut result_sensors_data: Vec<SensorData> = Vec::new();
      while let Ok(sensor_data) = sensors_data.try_next().await {
            match sensor_data {
                  Some(data) => result_sensors_data.push(data),
                  None => {
                        break;
                  }
            };
      }

      Ok(Json(
            GetSensorResponse { data: result_sensors_data }
      ))
}