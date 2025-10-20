use std::{collections::HashMap, sync::Arc};
use mongodb::Collection;
use tokio::sync::RwLock;
use crate::constants::model;

pub type CollectionsData = Arc<RwLock<HashMap<String, Collection<model::SensorData>>>>;