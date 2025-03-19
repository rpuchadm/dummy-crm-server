use std::env;

use serde::{Deserialize, Serialize};
use sqlx::{Decode, FromRow, postgres};

#[derive(Serialize, Deserialize, Clone, FromRow)]
pub struct ClienteRequest {
    pub user_id: i32,
    pub nombre: String,
    pub email: String,
    pub telefono: Option<String>,
    pub direccion: Option<String>,
}

#[derive(Serialize, Clone, FromRow)]
pub struct Cliente {
    id: i32,
    user_id: i32,
    nombre: String,
    email: String,
    telefono: Option<String>,
    direccion: Option<String>,
    fecha_registro: chrono::NaiveDateTime,
}

pub async fn postgres_get_clientes(
    pool: &sqlx::Pool<sqlx::Postgres>,
) -> Result<Vec<Cliente>, sqlx::Error> {
    let clientes = sqlx::query_as::<_, Cliente>(
        "SELECT
            id, user_id, nombre, email, telefono, direccion, fecha_registro
        FROM clientes",
    )
    .fetch_all(pool)
    .await?;

    Ok(clientes)
}

pub async fn postgres_get_cliente_by_user_id(
    pool: &sqlx::Pool<sqlx::Postgres>,
    user_id: i32,
) -> Result<Option<Cliente>, sqlx::Error> {
    let cliente: Option<Cliente> = sqlx::query_as::<_, Cliente>(
        "SELECT
            id, user_id, nombre, email, telefono, direccion, fecha_registro
        FROM clientes
        WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    Ok(cliente)
}

pub async fn postgres_get_cliente_by_id(
    pool: &sqlx::Pool<sqlx::Postgres>,
    id: i32,
) -> Result<Cliente, sqlx::Error> {
    let cliente: Cliente = sqlx::query_as::<_, Cliente>(
        "SELECT
            id, user_id, nombre, email, telefono, direccion, fecha_registro
        FROM clientes
        WHERE id = $1",
    )
    .bind(id)
    .fetch_one(pool)
    .await?;

    Ok(cliente)
}

pub async fn postgres_create_cliente(
    pool: &sqlx::Pool<sqlx::Postgres>,
    cliente: ClienteRequest,
) -> Result<Cliente, sqlx::Error> {
    let new_cliente = sqlx::query_as::<_, Cliente>(
        "INSERT INTO clientes (user_id, nombre, email, telefono, direccion)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, user_id, nombre, email, telefono, direccion, fecha_registro",
    )
    .bind(cliente.user_id)
    .bind(cliente.nombre)
    .bind(cliente.email)
    .bind(cliente.telefono)
    .bind(cliente.direccion)
    .fetch_one(pool)
    .await?;

    Ok(new_cliente)
}

pub async fn postgres_update_cliente(
    pool: &sqlx::Pool<sqlx::Postgres>,
    cliente: ClienteRequest,
    user_id: i32,
) -> Result<Cliente, sqlx::Error> {
    let updated_cliente = sqlx::query_as::<_, Cliente>(
        "UPDATE clientes
        SET nombre = $1, email = $2, telefono = $3, direccion = $4
        WHERE user_id = $5
        RETURNING id, user_id, nombre, email, telefono, direccion, fecha_registro",
    )
    .bind(cliente.nombre)
    .bind(cliente.email)
    .bind(cliente.telefono)
    .bind(cliente.direccion)
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    Ok(updated_cliente)
}

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
