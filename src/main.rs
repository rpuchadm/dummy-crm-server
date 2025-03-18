use chrono::prelude::*;
use redis::AsyncCommands;
use reqwest::Certificate;
use reqwest::Client;
use rocket::form::Form;
use rocket::http::Status;
use rocket::outcome::Outcome;
use rocket::request::{self, FromRequest, Request};
use rocket::response::status::NotFound;
use rocket::serde::json::Json;
use rocket::serde::{Deserialize, Serialize};
use rocket::{State, delete, get, launch, post, routes};
use rocket::{catch, catchers, put};
use rocket_cors::{AllowedHeaders, AllowedOrigins, CorsOptions};
// put
use sqlx::{Decode, FromRow, postgres};
use std::collections::HashMap;
use std::env;

mod articulos;
mod clientes;
mod postgresini;
mod sesion;

use articulos::{
    Articulo, ArticuloRequest, postgres_create_articulo, postgres_get_articulo_by_id,
    postgres_get_articulos, postgres_update_articulo,
};
use clientes::{Cliente, postgres_get_cliente_by_id};
use sesion::{AuthProfile, redis_get_session_by_token, redis_set_session_by_token};

struct AppState {
    pool: sqlx::Pool<sqlx::Postgres>,
    redis_connection_string: String,
}

#[launch]
async fn rocket() -> _ {
    let redis_password = std::env::var("REDIS_PASSWORD").unwrap_or_default();
    let redis_host = std::env::var("REDIS_SERVICE").unwrap_or_default();
    let redis_port = std::env::var("REDIS_PORT").unwrap_or_default();

    if redis_password.is_empty() || redis_host.is_empty() || redis_port.is_empty() {
        eprintln!("Error REDIS_PASSWORD, REDIS_SERVICE or REDIS_PORT is empty");
        std::process::exit(1);
    }

    let redis_connection_string =
        format!("redis://:{}@{}:{}/", redis_password, redis_host, redis_port);

    //print!("redis_connection_string: {}\n", redis_connection_string);

    // sacamos de env POSTGRES_DB
    let postgres_db =
        env::var("POSTGRES_DB").expect("La variable de entorno POSTGRES_DB no está definida");
    let postgres_user =
        env::var("POSTGRES_USER").expect("La variable de entorno POSTGRES_USER no está definida");
    let postgres_password = env::var("POSTGRES_PASSWORD")
        .expect("La variable de entorno POSTGRES_PASSWORD no está definida");
    let postgres_host = env::var("POSTGRES_SERVICE")
        .expect("La variable de entorno POSTGRES_SERVICE no está definida");

    let postgres_url = format!(
        "postgresql://{}:{}@{}:5432/{}",
        postgres_user, postgres_password, postgres_host, postgres_db
    );

    println!("Postgres URL: {}", postgres_url);

    let pool: sqlx::Pool<sqlx::Postgres> = sqlx::postgres::PgPool::connect(postgres_url.as_str())
        .await
        .or_else(|err| {
            eprintln!("Error connecting to the database: {:?}", err);
            Err(err)
        })
        .unwrap();

    postgresini::initialization(pool.clone()).await;

    let cors = cors_options().to_cors().expect("Error al configurar CORS");

    rocket::build()
        .manage(AppState {
            pool,
            redis_connection_string,
        })
        .mount(
            "/",
            routes![
                auth,
                getarticulo,
                getarticulos,
                healthz,
                postarticulo,
                postprofile,
                profile,
                profiles,
                putarticulo,
                putprofile,
                authback,
            ],
        )
        .register("/", catchers![not_found])
        .attach(cors)
}

struct BearerToken(String);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for BearerToken {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        if let Some(auth_header) = request.headers().get_one("Authorization") {
            if auth_header.starts_with("Bearer ") {
                let token = auth_header[7..].to_string();
                return Outcome::Success(BearerToken(token));
            }
        }
        Outcome::Error((Status::Unauthorized, ()))
    }
}

#[get("/healthz")]
async fn healthz() -> &'static str {
    "OK"
}

#[catch(404)]
fn not_found(req: &Request) -> NotFound<String> {
    // Registrar el error 404 en los logs
    eprintln!("Ruta no encontrada: {}", req.uri());

    // Devolver una respuesta 404 personalizada
    NotFound(format!("Lo siento, la ruta '{}' no existe.", req.uri()))
}

fn cors_options() -> CorsOptions {
    let allowed_origins = AllowedOrigins::some_exact(&["http://localhost:5173/"]);

    // You can also deserialize this
    rocket_cors::CorsOptions {
        allowed_origins,
        allowed_methods: vec![
            rocket::http::Method::Delete,
            rocket::http::Method::Get,
            rocket::http::Method::Post,
            rocket::http::Method::Put,
            rocket::http::Method::Options,
        ]
        .into_iter()
        .map(From::from)
        .collect(),
        allowed_headers: AllowedHeaders::some(&["Authorization", "Accept", "Content-Type"]),
        allow_credentials: true,
        ..Default::default()
    }
}

async fn auth_profile(token: BearerToken) -> Result<Option<AuthProfile>, Status> {
    let authprofile_url = env::var("AUTH_PROFILE_URL")
        .expect("La variable de entorno AUTH_PROFILE_URL no está definida");

    let client = Client::builder()
        .build()
        .expect("No se pudo crear el cliente HTTP");

    let response = client
        .get(authprofile_url)
        .header("Authorization", format!("Bearer {}", token.0))
        .send()
        .await
        .map_err(|e| {
            eprintln!("Error getting profile: {:?}", e);
            Status::InternalServerError
        })?;

    if response.status().is_success() {
        let profile: AuthProfile = response.json().await.map_err(|e| {
            eprintln!("Error parsing profile: {:?}", e);
            Status::InternalServerError
        })?;

        Ok(Some(profile))
    } else {
        eprintln!("auth_profile response status: {}", response.status());
        Err(Status::InternalServerError)
    }
}

#[derive(Serialize, Deserialize)]
struct AuthResponse {
    status: String,
}

#[get("/auth")]
async fn auth(
    state: &rocket::State<AppState>,
    token: BearerToken,
) -> Result<Json<AuthResponse>, Status> {
    if token.0.is_empty() {
        return Err(Status::Unauthorized);
    }

    let token_str = token.0.clone();

    let redis_client =
        redis::Client::open(state.redis_connection_string.clone()).map_err(|err| {
            eprintln!("Error connecting to redis: {:?}", err);
            Status::InternalServerError
        })?;

    let profile = redis_get_session_by_token(&redis_client, &token_str)
        .await
        .map_err(|e| {
            eprintln!("Error getting session: {:?}", e);
            Status::InternalServerError
        })?;

    if profile.is_some() {
        return Ok(Json(AuthResponse {
            status: "success".to_string(),
        }));
    }

    let profile = match auth_profile(token).await {
        Ok(profile) => profile,
        Err(e) => {
            eprintln!("Error getting profile: {:?}", e);
            return Err(Status::InternalServerError);
        }
    };

    if profile.is_none() {
        return Err(Status::Unauthorized);
    }

    let profile = profile.unwrap();

    if profile.user_id == 0 {
        return Err(Status::Forbidden);
    }

    redis_set_session_by_token(&redis_client, &token_str, &profile)
        .await
        .map_err(|e| {
            eprintln!("Error setting session: {:?}", e);
            Status::InternalServerError
        })?;

    Ok(Json(AuthResponse {
        status: "success".to_string(),
    }))
}

#[get("/articulos")]
async fn getarticulos(
    state: &rocket::State<AppState>,
    token: BearerToken,
) -> Result<Json<Vec<Articulo>>, Status> {
    let profile = match auth_profile(token).await {
        Ok(profile) => profile,
        Err(e) => {
            eprintln!("Error getting profile: {:?}", e);
            return Err(Status::InternalServerError);
        }
    };

    if profile.is_none() {
        return Err(Status::Unauthorized);
    }

    let profile = profile.unwrap();

    if profile.user_id == 0 {
        return Err(Status::Forbidden);
    }

    let pool = state.pool.clone();
    let varticulos = postgres_get_articulos(&pool).await.map_err(|e| {
        eprintln!("Error getting articles: {:?}", e);
        Status::InternalServerError
    })?;

    Ok(Json(varticulos))
}

#[get("/articulo/<id>")]
async fn getarticulo(
    state: &rocket::State<AppState>,
    token: BearerToken,
    id: i32,
) -> Result<Json<Articulo>, Status> {
    let profile = match auth_profile(token).await {
        Ok(profile) => profile,
        Err(e) => {
            eprintln!("Error getting profile: {:?}", e);
            return Err(Status::InternalServerError);
        }
    };

    if profile.is_none() {
        return Err(Status::Unauthorized);
    }

    let profile = profile.unwrap();

    if profile.user_id == 0 {
        return Err(Status::Forbidden);
    }

    let pool = state.pool.clone();
    let articulo = postgres_get_articulo_by_id(&pool, id).await.map_err(|e| {
        eprintln!("Error getting article: {:?}", e);
        Status::InternalServerError
    })?;

    Ok(Json(articulo))
}

#[post("/articulo/<id>", data = "<articulo>")]
async fn postarticulo(
    state: &rocket::State<AppState>,
    token: BearerToken,
    articulo: Json<ArticuloRequest>,
    id: i32,
) -> Result<Json<Articulo>, Status> {
    let profile = match auth_profile(token).await {
        Ok(profile) => profile,
        Err(e) => {
            eprintln!("Error getting profile: {:?}", e);
            return Err(Status::InternalServerError);
        }
    };

    if profile.is_none() {
        return Err(Status::Unauthorized);
    }

    let profile = profile.unwrap();

    if profile.user_id == 0 {
        return Err(Status::Forbidden);
    }

    let pool = state.pool.clone();
    let articulo = articulo.into_inner();

    // si id no es 0 da error
    if articulo.id != 0 || id != 0 {
        eprintln!("Error creating article: id must be 0");
        return Err(Status::BadRequest);
    }

    let new_articulo = postgres_create_articulo(&pool, articulo)
        .await
        .map_err(|e| {
            eprintln!("Error creating article: {:?}", e);
            Status::InternalServerError
        })?;

    Ok(Json(new_articulo))
}

#[put("/articulo/<id>", data = "<articulo>")]
async fn putarticulo(
    state: &rocket::State<AppState>,
    token: BearerToken,
    articulo: Json<ArticuloRequest>,
    id: i32,
) -> Result<Json<Articulo>, Status> {
    let profile = match auth_profile(token).await {
        Ok(profile) => profile,
        Err(e) => {
            eprintln!("Error getting profile: {:?}", e);
            return Err(Status::InternalServerError);
        }
    };

    if profile.is_none() {
        return Err(Status::Unauthorized);
    }

    let profile = profile.unwrap();

    if profile.user_id == 0 {
        return Err(Status::Forbidden);
    }

    let pool = state.pool.clone();
    let articulo = articulo.into_inner();

    // si id es 0 da error
    if articulo.id == 0 || id != articulo.id {
        eprintln!("Error updating article: id must not be 0 and equal to the URL id");
        return Err(Status::BadRequest);
    }

    let new_articulo = postgres_update_articulo(&pool, articulo, id)
        .await
        .map_err(|e| {
            eprintln!("Error updating article: {:?}", e);
            Status::InternalServerError
        })?;

    Ok(Json(new_articulo))
}

#[get("/profile/<id>")]
async fn profile(
    state: &State<AppState>,
    token: BearerToken,
    id: i32,
) -> Result<Json<Cliente>, Status> {
    let profile = match auth_profile(token).await {
        Ok(profile) => profile,
        Err(e) => {
            eprintln!("Error getting profile: {:?}", e);
            return Err(Status::InternalServerError);
        }
    };

    if profile.is_none() {
        return Err(Status::Unauthorized);
    }

    let profile = profile.unwrap();

    if profile.user_id == 0 {
        return Err(Status::Forbidden);
    }

    let pool = state.pool.clone();

    let cliente = postgres_get_cliente_by_id(&pool, id).await.map_err(|e| {
        eprintln!("Error getting client: {:?}", e);
        Status::InternalServerError
    })?;

    Ok(Json(cliente))
}

#[get("/profiles")]
async fn profiles(
    state: &State<AppState>,
    token: BearerToken,
) -> Result<Json<Vec<Cliente>>, Status> {
    let profile = match auth_profile(token).await {
        Ok(profile) => profile,
        Err(e) => {
            eprintln!("Error getting profile: {:?}", e);
            return Err(Status::InternalServerError);
        }
    };

    if profile.is_none() {
        return Err(Status::Unauthorized);
    }

    let profile = profile.unwrap();

    if profile.user_id == 0 {
        return Err(Status::Forbidden);
    }

    let pool = state.pool.clone();

    let clientes = clientes::postgres_get_clientes(&pool).await.map_err(|e| {
        eprintln!("Error getting clients: {:?}", e);
        Status::InternalServerError
    })?;

    Ok(Json(clientes))
}

#[post("/profile", data = "<cliente>")]
async fn postprofile(
    state: &State<AppState>,
    token: BearerToken,
    cliente: Json<clientes::ClienteRequest>,
) -> Result<Json<Cliente>, Status> {
    let profile = match auth_profile(token).await {
        Ok(profile) => profile,
        Err(e) => {
            eprintln!("Error getting profile: {:?}", e);
            return Err(Status::InternalServerError);
        }
    };

    if profile.is_none() {
        return Err(Status::Unauthorized);
    }

    let profile = profile.unwrap();

    if profile.user_id == 0 {
        return Err(Status::Forbidden);
    }

    let pool = state.pool.clone();
    let cliente = cliente.into_inner();

    let new_cliente = clientes::postgres_create_cliente(&pool, cliente)
        .await
        .map_err(|e| {
            eprintln!("Error creating client: {:?}", e);
            Status::InternalServerError
        })?;

    Ok(Json(new_cliente))
}

#[put("/profile/<user_id>", data = "<cliente>")]
async fn putprofile(
    state: &State<AppState>,
    token: BearerToken,
    cliente: Json<clientes::ClienteRequest>,
    user_id: i32,
) -> Result<Json<Cliente>, Status> {
    let profile = match auth_profile(token).await {
        Ok(profile) => profile,
        Err(e) => {
            eprintln!("Error getting profile: {:?}", e);
            return Err(Status::InternalServerError);
        }
    };

    if profile.is_none() {
        return Err(Status::Unauthorized);
    }

    let profile = profile.unwrap();

    if profile.user_id == 0 {
        return Err(Status::Forbidden);
    }

    let pool = state.pool.clone();
    let cliente = cliente.into_inner();

    let new_cliente = clientes::postgres_update_cliente(&pool, cliente, user_id)
        .await
        .map_err(|e| {
            eprintln!("Error updating client: {:?}", e);
            Status::InternalServerError
        })?;

    Ok(Json(new_cliente))
}

#[derive(Serialize, Deserialize)]
struct AccessTokenResponse {
    access_token: String,
    token_type: String,
    expires_in: i32,
}

#[get("/authback/<code>")]
async fn authback(code: &str) -> Result<Option<Json<AccessTokenResponse>>, Status> {
    //saca authback_url de env
    let authback_url = env::var("AUTH_ACCESSTOKEN_URL")
        .expect("La variable de entorno AUTH_ACCESSTOKEN_URL no está definida");

    let client_id =
        env::var("CLIENT_ID").expect("La variable de entorno CLIENT_ID no está definida");

    let redirect_uri =
        env::var("REDIRECT_URI").expect("La variable de entorno REDIRECT_URI no está definida");

    // Crear un cliente que no verifique los certificados SSL
    let client = Client::builder()
        //.danger_accept_invalid_certs(true) // Desactiva la verificación SSL
        .build()
        .map_err(|e| {
            eprintln!("Error building client: {:?}", e);
            Status::InternalServerError
        })?;

    let response = client
        .post(authback_url)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(format!(
            "grant_type=authorization_code&code={}&client_id={}&redirect_uri={}",
            code, client_id, redirect_uri
        ))
        .send()
        .await
        .map_err(|e| {
            eprintln!("Error getting access token: {:?}", e);
            Status::InternalServerError
        })?;

    let response_text = response.text().await.map_err(|e| {
        eprintln!("Error getting response text: {:?}", e);
        Status::InternalServerError
    })?;

    let response: AccessTokenResponse = serde_json::from_str(&response_text).map_err(|e| {
        eprintln!(
            "Error parsing access token response: {:?} {}",
            e, response_text
        );
        Status::InternalServerError
    })?;

    Ok(Some(Json(response)))
}
