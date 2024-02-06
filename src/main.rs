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
use pyo3::types::IntoPyDict;

#[derive(Deserialize)]
struct Payload {
    excelData: Vec<HashMap<String, String>>,
    formatters: HashMap<String, Formatter>,
}

#[derive(Deserialize)]
struct Formatter {
    func: String,
}

// ESTO ES CODIGO MAS LISTO PARA PRODUCTIVO, SE COMENTA PARA LA POC
// fn execute_python_code(py_code: &str, value: &str) -> PyResult<()> {
//     // print!("{:?}", py_code);
//     // print!("{:?}", value);
//     let code = format!("{}\nresult = transform(value)", py_code);
//     pyo3::prepare_freethreaded_python();
//     Python::with_gil(|py| {
//         let full_code = format!("{}\ncleaned_string = clean_string(input_string)", py_code);
//         print!("{:?}", full_code);
//         let fun: Py<PyAny> = PyModule::from_code(py, &code, "", "")?
//             .getattr("example")?
//             .into();
//         let py_value = PyString::new(py, value);
//         fun.call1(py, (py_value,))?;
//         Ok(())
//     })
// }

// ESTO ES CODIGO MAS LISTO PARA PRODUCTIVO, SE COMENTA PARA LA POC
// #[post("/", format = "json", data = "<payload>")]
// fn receive_code(payload: Json<Payload>) -> String {
//     let excel_data = &payload.excelData;
//     let formatters = &payload.formatters;
//     print!("{:?}", excel_data);
//     for row in excel_data {
//         for (key, value) in row {
//             if let Some(formatter) = formatters.get(key) {
//                 match execute_python_code(&formatter.func, value) {
//                     Ok(transformed_value) => println!("Transformed"),
//                     Err(e) => println!("Error executing formatter for {}: {}", key, e),
//                 }
//             }
//         }
//     }

//     "Data processed successfully.".to_string()
// }
const TRANSFORM_FUNCTION: &str = r#"
def transform(value):
    return value.upper()
"#;

const CLEAN_STRING_FUNCTION: &str = r#"
def transform(value):
    return value.replace('.', '').replace('-', '')
"#;

// Create a mapping of keys to Python functions
fn get_formatter_functions() -> HashMap<&'static str, &'static str> {
    let mut formatters = HashMap::new();
    formatters.insert("Nombre Paciente", TRANSFORM_FUNCTION);
    formatters.insert("Apellidos", TRANSFORM_FUNCTION);
    formatters.insert("RUN Pacientes", CLEAN_STRING_FUNCTION);
    formatters
}

fn execute_python_code(py_code: &str, value: &str) -> Result<Value, String> {
    pyo3::prepare_freethreaded_python();
    Python::with_gil(|py| {
        let locals = PyDict::new(py);
        locals
            .set_item("value", PyString::new(py, &value.to_string()))
            .unwrap();
        py.run(py_code, None, Some(locals))
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
    let formatters = get_formatter_functions(); // Use hardcoded formatters
    let mut results = Vec::new(); // to collect processed rows

    for row in excel_data {
        let mut processed_row = HashMap::new();
        for (key, value) in row {
            if let Some(py_code) = formatters.get(key.as_str()) {
                match execute_python_code(py_code, value) {
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
