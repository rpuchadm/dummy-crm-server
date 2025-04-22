use std::env;

use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct AccessTokenResponse {
    access_token: String,
    token_type: String,
    expires_in: i32,
}

pub async fn corp_service_user_token() -> Result<String, Box<dyn std::error::Error>> {
    // Obtener variables de entorno
    let authback_url = env::var("AUTH_ACCESSTOKEN_URL")
        .map_err(|_| "La variable AUTH_ACCESSTOKEN_URL no está definida")?;

    let client_id = env::var("CLIENT_ID").map_err(|_| "La variable CLIENT_ID no está definida")?;

    let client_secret =
        env::var("CLIENT_SECRET").map_err(|_| "La variable CLIENT_SECRET no está definida")?;

    // Debug (opcional)
    println!(
        "Obteniendo token de {} para client_id: {}",
        authback_url, client_id
    );

    // Crear cliente y hacer la petición
    let client = reqwest::Client::new();
    let response = client
        .post(&authback_url)
        .basic_auth(client_id, Some(client_secret))
        .form(&[("grant_type", "client_credentials")])
        .send()
        .await
        .map_err(|e| format!("Fallo en la petición HTTP: {}", e))?;

    // Manejar errores HTTP
    if !response.status().is_success() {
        let status = response.status();
        let error_body = response.text().await.unwrap_or_default();
        return Err(format!("Error HTTP {}: {}", status, error_body).into());
    }

    // Parsear respuesta JSON
    let response_json: AccessTokenResponse = response
        .json()
        .await
        .map_err(|e| format!("Error parseando JSON: {}", e))?;

    Ok(response_json.access_token)
}

#[derive(Serialize, Deserialize)]
pub struct UserData {
    pub id: i32,
    pub dni: String,
    pub nombre: String,
    pub apellidos: String,
    pub email: String,
    pub telefono: String,
}

pub async fn corp_service_userdata_by_id(
    user_id: i32,
) -> Result<Option<UserData>, Box<dyn std::error::Error>> {
    // Obtener token - ahora maneja el error con ?
    let token = corp_service_user_token().await?;

    // Obtener URL del servicio - con manejo de error
    let corp_url = env::var("CORP_SERVICE_USERDATA_URL")
        .map_err(|_| "La variable de entorno CORP_SERVICE_USERDATA_URL no está definida")?;

    // Construir URL completa
    let url = format!("{}/person/{}", corp_url, user_id);
    println!("Consultando: {}", url); // Debug opcional

    // Crear cliente y hacer la petición
    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| format!("Error al realizar la petición: {}", e))?;

    // Manejar respuesta
    match response.status() {
        StatusCode::OK => {
            let response_json = response
                .json()
                .await
                .map_err(|e| format!("Error al parsear respuesta: {}", e))?;
            Ok(Some(response_json))
        }
        StatusCode::NOT_FOUND => Ok(None), // Usuario no encontrado
        status => Err(format!(
            "Error inesperado del servidor: {} - {}",
            status,
            response.text().await.unwrap_or_default()
        )
        .into()),
    }
}
