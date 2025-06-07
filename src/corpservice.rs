use std::env;

use log;
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
    let auth_access_token_url = env::var("AUTH_ACCESSTOKEN_CLIENT_URL")
        .map_err(|_| "La variable AUTH_ACCESSTOKEN_CLIENT_URL no está definida")?;

    let client_id = env::var("CLIENT_ID").map_err(|_| "La variable CLIENT_ID no está definida")?;

    let client_secret =
        env::var("CLIENT_SECRET").map_err(|_| "La variable CLIENT_SECRET no está definida")?;

    // Debug (opcional)
    println!(
        "Obteniendo token de {} para client_id: {} client_secret: {}",
        auth_access_token_url, client_id, client_secret
    );

    // Crear cliente y hacer la petición
    let client = reqwest::Client::new();
    let response = client
        .post(&auth_access_token_url)
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
pub struct PersonData {
    pub id: i32,
    pub dni: String,
    pub nombre: String,
    pub apellidos: String,
    pub email: String,
    pub telefono: String,
}
#[derive(Serialize, Deserialize)]
pub struct AppData {
    pub id: i32,
    pub client_id: String,
    pub client_url: String,
}
#[derive(Serialize, Deserialize)]
pub struct PersonAppData {
    pub id: i32,
    pub person_id: i32,
    pub auth_client_id: i32,
    //pub created_at: String,
    pub profile: String,
}
#[derive(Serialize, Deserialize)]
pub struct UserData {
    pub person: PersonData,
    pub lapp: Vec<AppData>,
    pub lpersonapp: Vec<PersonAppData>,
}

pub async fn corp_service_userdata_by_id(
    user_id: i32,
) -> Result<Option<UserData>, Box<dyn std::error::Error>> {
    // Obtener token
    let token = corp_service_user_token().await?;
    println!(
        "INFO fn corp_service_userdata_by_id Token obtenido: {}",
        token
    );

    // Obtener URL del servicio
    let corp_url = env::var("CORP_SERVICE_USERDATA_URL")
        .map_err(|_| "La variable de entorno CORP_SERVICE_USERDATA_URL no está definida")?;

    // Construir URL completa
    let url = format!("{}/person/{}", corp_url, user_id);
    println!("corp_service_userdata_by_id Consultando: {}", url);

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
            let response_text = response.text().await.unwrap_or_default();

            match serde_json::from_str::<UserData>(&response_text) {
                Ok(response_json) => Ok(Some(response_json)),
                Err(e) => {
                    log::error!(
                        "Error al parsear respuesta: {}\nContenido JSON: {}",
                        e,
                        response_text
                    );
                    Err(format!("Error al parsear respuesta: {}", e).into())
                }
            }
        }
        StatusCode::NOT_FOUND => Ok(None),
        status => {
            let error_body = response.text().await.unwrap_or_default();
            log::error!(
                "Error inesperado del servidor: {}\nContenido respuesta: {}",
                status,
                error_body
            );
            Err(format!("Error inesperado del servidor: {} - {}", status, error_body).into())
        }
    }
}
