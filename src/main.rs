// src/main.rs
use log::{error, info};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyString};
use rocket::{serde::json::Json, State};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Mutex;

#[macro_use]
extern crate rocket;
extern crate log;

#[post("/", format = "json", data = "<input_data>")]
async fn process_data(input_data: Json<Vec<Value>>, state: &State<AppState>) -> Json<Vec<Value>> {
    let mut data = input_data.into_inner();
    let transformations = match state.transformations.lock() {
        Ok(t) => t,
        Err(e) => {
            error!("Error acquiring lock: {}", e);
            return Json(vec![]); // Consider handling this more gracefully
        }
    };

    for user in &mut data {
        if let Value::Object(ref mut user_obj) = user {
            for (key, transform) in &*transformations {
                let key_string = key.to_string(); // Convert key to String once
                if let Some(value) = user_obj.get(&key_string) {
                    // Use a reference to the String
                    match transform(value) {
                        Ok(new_value) => {
                            // Now you can use the owned String directly
                            user_obj.insert(key_string, new_value);
                        }
                        Err(e) => {
                            error!("Error transforming {}: {}", key, e);
                        }
                    }
                }
            }
        }
    }
    Json(data)
}

struct AppState {
    transformations: Mutex<HashMap<&'static str, fn(&Value) -> Result<Value, String>>>,
}

#[launch]
fn rocket() -> _ {
    env_logger::init();

    let transformations: HashMap<&'static str, fn(&Value) -> Result<Value, String>> =
        HashMap::new();
    let app_state = AppState {
        transformations: Mutex::new(transformations),
    };

    rocket::build()
        .manage(app_state)
        .mount("/", routes![process_data])
}
