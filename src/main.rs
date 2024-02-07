// src/main.rs
use log::{error, info};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyString};
use rocket::{serde::json::Json, State};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

#[macro_use]
extern crate rocket;

#[derive(Deserialize)]
struct Payload {
    excelData: Vec<HashMap<String, String>>,
    formatters: HashMap<String, Formatter>,
}

#[derive(Deserialize)]
struct Formatter {
    func: String,
}

// ************************* USAR PARA PROBAR FUNCIONES O DEMOS *************************
// // Funcion para demos
// const TRANSFORM_FUNCTION: &str = r#"
// def transform(value):
//     return value.upper()
// "#;
// // Funcion para demos
// const CLEAN_STRING_FUNCTION: &str = r#"
// def transform(value):
//     return value.replace('.', '').replace('-', '')
// "#;

// // Funcion para demos
// fn get_formatter_functions() -> HashMap<&'static str, &'static str> {
//     let mut formatters = HashMap::new();
//     formatters.insert("Nombre Paciente", TRANSFORM_FUNCTION);
//     formatters.insert("Apellidos", TRANSFORM_FUNCTION);
//     formatters.insert("RUN Pacientes", CLEAN_STRING_FUNCTION);
//     formatters
// }
// ************************* USAR PARA PROBAR FUNCIONES O DEMOS *************************

fn deserialize_python_code(serialized_code: &str) -> String {
    let mut deserialized_code = serialized_code
        .replace("\\n", "\n")
        .replace("\\'", "'")
        .replace("\\\"", "\"")
        .replace("\\\\", "\\");

    if deserialized_code.starts_with("'") && deserialized_code.ends_with("'") {
        deserialized_code = deserialized_code[1..deserialized_code.len() - 1].to_string();
    }

    deserialized_code
}

fn execute_python_code(py_code: &str, value: &str) -> Result<Value, String> {
    pyo3::prepare_freethreaded_python();
    let py_code = deserialize_python_code(py_code);
    print!("codigo python: {}", py_code);
    Python::with_gil(|py| {
        let locals = PyDict::new(py);
        locals
            .set_item("value", PyString::new(py, &value.to_string()))
            .unwrap();
        py.run(&py_code, None, Some(locals))
            .map_err(|e| e.to_string())?;

        let transform_func = locals
            .get_item("transform")
            .or_else(|_| Err("Function 'transform' not found in Python locals".to_string()))?;

        let transformed_value = transform_func
            .unwrap()
            .call1((value.to_string(),))
            .map_err(|e| e.to_string())?
            .extract::<String>()
            .map_err(|e| e.to_string())?;

        Ok(json!(transformed_value))
    })
}

#[post("/", format = "json", data = "<payload>")]
fn receive_code(payload: Json<Payload>) -> Json<Value> {
    let excel_data = &payload.excelData;
    // descomentar para demos
    // let formatters = get_formatter_functions();
    let formatters = &payload.formatters;
    let mut results = Vec::new();

    for row in excel_data {
        let mut processed_row = HashMap::new();
        for (key, value) in row {
            if let Some(py_code) = formatters.get(key.as_str()) {
                match execute_python_code(&py_code.func, value) {
                    Ok(transformed_value) => {
                        println!("Transformed {}: {}", key, transformed_value);
                        processed_row.insert(key.clone(), transformed_value);
                    }
                    Err(e) => {
                        println!("Error executing formatter for {}: {}", key, e);
                        processed_row.insert(key.clone(), serde_json::Value::String(value.clone()));
                    }
                }
            } else {
                processed_row.insert(key.clone(), serde_json::Value::String(value.clone()));
            }
        }
        results.push(processed_row);
    }

    Json(json!({"excelData": results}))
}

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![receive_code])
}
