use dotenvy::dotenv;
use gaia_sensor_logger::constants::{config, model};
use mongodb::{
    Collection,
    options::{ClientOptions, TimeseriesOptions},
};
use rumqttc::{AsyncClient, Event, MqttOptions, QoS};
use sqlx::postgres::PgPoolOptions;
use std::{collections::HashMap, env};
use tokio;
use chrono::Local;

#[tokio::main]
async fn main() -> Result<(), String> {
    // --- --- --- --- Setup Dotenv
    dotenv().ok(); // Reads the .env file

    // --- --- --- --- Conenct to MongoDB
    println!("[MongoDB] Connecting to MongoDB...");
    let mongo_url = env::var("MONGO_URL").unwrap();
    let mongo_options = match ClientOptions::parse(&mongo_url).await {
        Ok(opt) => opt,
        Err(err) => {
            return Err(err.to_string());
        }
    };
    let mongo_client = mongodb::Client::with_options(mongo_options).unwrap();
    let db = mongo_client.database("gaia-sensor-db");
    let available_collections = match db.list_collection_names().await {
        Ok(data) => data,
        Err(err) => {
            return Err(err.to_string());
        }
    };

    println!("[MongoDB] MongoDB Connected! ✅");

    // --- --- --- --- Connect to PostgreSQL
    println!("[PostgreSQL] Connecting to PostgreSQL...");
    let pg_url = env::var("DATABASE_URL").unwrap();
    let pg_pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(pg_url.as_str())
        .await
        .unwrap();
    println!("[PostgreSQL] PostgreSQL Connected! ✅");

    // --- --- --- --- Verify and create collections if not exists
    println!("[MongoDB] Verifying MongoDB collections...");
    let mut collections: HashMap<String, Collection<model::SensorData>> = HashMap::new();
    // Iterate each collections that we want to use for logging data
    for topic_name in config::DB_COLLECTIONS.keys() {
        let collection_name: &String = config::DB_COLLECTIONS.get(topic_name).unwrap();
        
        // If there's a missing collection
        if !available_collections.contains(collection_name) {
            // Create a collection options
            let time_series_options = TimeseriesOptions::builder()
                .time_field("timestamp")
                .meta_field(Some(String::from("metadata")))
                .granularity(Some(mongodb::options::TimeseriesGranularity::Minutes))
                .build();
    
            // Add the collection to the database
            db.create_collection(collection_name)
                .timeseries(time_series_options)
                .expire_after_seconds(std::time::Duration::from_secs(60 * 60 * 24)) // Expire after an hour
                .await
                .unwrap();
        }


        // Get the collection value
        let collection: Collection<model::SensorData> = db.collection(collection_name.as_str());
        collections.insert(topic_name.clone(), collection);
    }

    println!("[MongoDB] MongoDB Collections has been set up! ✅");

    // --- --- --- --- Get all of the devices data
    println!("[PostgreSQL] Get all devices data from PostgreSQL...");
    let devices_id: Vec<String> = sqlx::query!("SELECT id FROM devices")
        .fetch_all(&pg_pool)
        .await
        .unwrap()
        .iter()
        .map(|item| item.id.clone())
        .collect::<Vec<String>>();

    if devices_id.len() == 0 {
        println!("[PostgreSQL] There's no device exists in database");
        println!("👋🏻 I'm leaving...");
        return Ok(());
    }
    println!("[PostgreSQL] All devices data has been retrieved ✅");

    // --- --- --- --- Connect to MQTT broker
    // Get the MQTT credentials
    let mqtt_username: String = env::var("SPECIAL_USER").unwrap();
    let mqtt_password: String = env::var("SPECIAL_PASS").unwrap();

    // Preparing MQTT connecttion settings
    let mqtt_url: String = env::var("MQTT_ADDRESS").unwrap();
    let mut mqttoptions = MqttOptions::new(mqtt_username.clone(), mqtt_url, 1883);
    mqttoptions.set_credentials(mqtt_username, mqtt_password);
    mqttoptions.set_keep_alive(std::time::Duration::from_secs(5));

    // Connect to the MQTT Broker
    println!("[MQTT] Setting up Connection to MQTT...");
    let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);

    // Subscribe to each devices and its subtopics
    println!("[MQTT] Subscribing Topics...");
    let max_iterations: usize = devices_id.len() * config::MQTT_SUBTOPICS.len();
    let mut current_percent: f32 = 0.0;
    let delta_percent: f32 = 100f32 / max_iterations as f32;
    for device_id in devices_id {
        for subtopic in config::MQTT_SUBTOPICS.iter() {
            current_percent += delta_percent;
            let topic: String = format!("{}/{}", device_id, subtopic);
            client.subscribe(topic, QoS::AtMostOnce).await.unwrap();
            print!("{}%\r", current_percent.to_string());
        }
    }

    println!("[MQTT] Listening to MQTT messages ✅");
    // --- --- --- --- Process MQTT events
    loop {
        let event = eventloop.poll().await.unwrap();
        if let Event::Incoming(incoming) = event {
            // Get Connect Message
            if let rumqttc::Packet::Connect(_) = incoming {
                println!("[MQTT] Connected!");
            } else if let rumqttc::Packet::Publish(publish) = incoming {
                let payload = String::from_utf8_lossy(&publish.payload);
                println!("[MQTT] Message on {:?}: {}", publish.topic, payload);

                if let Ok(value) = payload.parse::<f32>() {
                    let subtopics: Vec<&str> = publish.topic.split("/").collect::<Vec<&str>>();
                    let subtopic: String = match subtopics.get(1) {
                        Some(data) => data.to_string(),
                        None => {
                            println!(
                                "[MQTT] Sir, I got a weird case where there's no '/' in the topic. But, you've said that it won't be possible"
                            );
                            continue;
                        }
                    };
                    let device_id = subtopics.first().unwrap();

                    let collection = match collections.get(&subtopic) {
                        Some(col) => col,
                        None => {
                            println!(
                                "[MQTT] Hello, sir. I got a weird case where the recieved topic from client is different from what we've planned. (ec, tds, ph, tempC)"
                            );
                            continue;
                        }
                    };

                    let sensor_data: model::SensorData = model::SensorData {
                        metadata: model::Metadata {
                            device_id: device_id.to_string(),
                        },
                        timestamp: Local::now().to_rfc3339(),
                        data: value,
                    };

                    let insert_result = collection.insert_one(sensor_data).await;

                    match insert_result {
                        Ok(_) => println!("[MQTT] Stored in MongoDB"),
                        Err(err) => println!(
                            "[MQTT] Error when storing data to MongoDB. Error: {}",
                            err.to_string()
                        ),
                    }
                } else {
                    println!("[MQTT] Invalid JSON format");
                }
            }
        }
    }
}
