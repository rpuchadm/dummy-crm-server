use std::env;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct AccessTokenResponse {
    access_token: String,
    token_type: String,
    expires_in: i32,
}

pub async fn corp_service_user_token() -> String {
    let authback_url = env::var("AUTH_ACCESSTOKEN_URL")
        .expect("La variable de entorno AUTH_ACCESSTOKEN_URL no está definida");

    let client_id =
        env::var("CLIENT_ID").expect("La variable de entorno CLIENT_ID no está definida");

    let client_secret =
        env::var("CLIENT_SECRET").expect("La variable de entorno CLIENT_SECRET no está definida");

    let client = reqwest::Client::new();
    let response = client
        .post(&authback_url)
        .basic_auth(client_id, Some(client_secret))
        .form(&[("grant_type", "client_credentials")])
        .send()
        .await
        .expect("Error al solicitar el token de acceso");

    let response_json: AccessTokenResponse = response.json().await.unwrap();
    response_json.access_token
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

pub async fn corp_service_userdata_by_id(user_id: i32) -> Option<UserData> {
    let token = corp_service_user_token().await;

    let corp_url = env::var("CORP_SERVICE_USERDATA_URL")
        .expect("La variable de entorno CORP_SERVICE_URL no está definida");

    let client = reqwest::Client::new();
    let response = client
        .get(&format!("{}/person/{}", corp_url, user_id))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .expect("Error al solicitar el usuario");

    if response.status().is_success() {
        let response_json: UserData = response.json().await.unwrap();
        Some(response_json)
    } else {
        None
    }
}
