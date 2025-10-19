use std::collections::HashMap;
use once_cell::sync::Lazy;

pub static DB_COLLECTIONS: Lazy<HashMap<String, String>> = Lazy::new(|| {
      let mut db_collections:  HashMap<String, String> = HashMap::new();
      db_collections.insert(String::from("ec"), String::from("sensor-ec"));
      db_collections.insert(String::from("tds"), String::from("sensor-tds"));
      db_collections.insert(String::from("tempC"), String::from("sensor-temp-c"));
      db_collections.insert(String::from("ph"), String::from("sensor-ph"));
      db_collections
});


pub static MQTT_SUBTOPICS: Lazy<Vec<String>> = Lazy::new(|| {
      let mut mqtt_topics:  Vec<String> = Vec::new();
      mqtt_topics.push(String::from("ec"));
      mqtt_topics.push(String::from("tds"));
      mqtt_topics.push(String::from("tempC"));
      mqtt_topics.push(String::from("ph"));
      mqtt_topics
});
