use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Serialize, Deserialize)]
pub struct IssueServicePostData {
    pub subject: String,
    pub description: String,
    pub project_id: i32,
    pub tracker_id: i32,
}

pub async fn issue_service_post(
    issue_service_post_data: IssueServicePostData,
    token: String,
) -> Result<String, Box<dyn std::error::Error>> {
    let issue_service_url = env::var("ISSUE_CREATE_URL")
        .map_err(|_| "La variable ISSUE_CREATE_URL no está definida")?;

    if issue_service_post_data.project_id == 0 {
        return Err("El project_id no puede ser 0".into());
    }

    if issue_service_post_data.tracker_id == 0 {
        return Err("El tracker_id no puede ser 0".into());
    }

    print!("issue_service_post_data: {:?}", issue_service_post_data);
    println!("token: {}", token);
    println!("issue_service_url: {}", issue_service_url);

    let client = reqwest::Client::new();

    let response = client
        .post(&issue_service_url)
        .header("Authorization", format!("Bearer {}", token))
        .json(&issue_service_post_data)
        .send()
        .await
        .map_err(|e| format!("Error al realizar la petición: {}", e))?;
    if !response.status().is_success() {
        return Err(format!("Error en la respuesta: {}", response.status()).into());
    }
    let response_text = response
        .text()
        .await
        .map_err(|e| format!("Error al leer la respuesta: {}", e))?;
    let response_json: serde_json::Value = serde_json::from_str(&response_text)
        .map_err(|e| format!("Error al parsear la respuesta JSON: {}", e))?;
    let issue_id = response_json
        .get("issue_id")
        .and_then(|v| v.as_i64())
        .ok_or("Error al obtener el issue_id de la respuesta")?;
    Ok(issue_id.to_string())
}
