use chrono::prelude::*;
use redis::AsyncCommands;
use rocket::FromForm;
use rocket::form::Form;
use rocket::http::Status;
use rocket::outcome::Outcome;
use rocket::request::{self, FromRequest, Request};
use rocket::serde::json::Json;
use rocket::serde::{Deserialize, Serialize};
use rocket::{State, delete, get, launch, post, routes}; // put
use sqlx::{Decode, FromRow, postgres};
use std::env;

mod articulos;
mod clientes;
mod postgresini;

use articulos::{Articulo, postgres_get_articulos};
use clientes::{Cliente, postgres_get_cliente_by_user_id};

struct AppState {
    pool: sqlx::Pool<sqlx::Postgres>,
}

// constante con el servidor de postgres
//const POSTGRES_SERVER: &str = "postgresql://myuser:mypassword@localhost:5432/mydatabase";

#[launch]
async fn rocket() -> _ {
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

    let pool: sqlx::Pool<sqlx::Postgres> = sqlx::postgres::PgPool::connect(postgres_url.as_str())
        .await
        .or_else(|err| {
            eprintln!("Error connecting to the database: {:?}", err);
            Err(err)
        })
        .unwrap();

    postgresini::initialization(pool.clone()).await;

    rocket::build()
        .manage(AppState { pool })
        .mount("/", routes![auth, getarticulos, healthz, profile])
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

const SUPER_SECRET: &str = "super_secret_token";

#[get("/auth")]
async fn auth(state: &rocket::State<AppState>, token: BearerToken) -> Result<String, Status> {
    if token.0 != SUPER_SECRET {
        eprintln!("Error invalid super secret token");
        return Err(Status::Unauthorized);
    }
    Ok("You are authorized!".to_string())
}

#[get("/articulos")]
async fn getarticulos(state: &rocket::State<AppState>) -> Result<Json<Vec<Articulo>>, Status> {
    let pool = state.pool.clone();
    let varticulos = postgres_get_articulos(&pool).await.map_err(|e| {
        eprintln!("Error getting articles: {:?}", e);
        Status::InternalServerError
    })?;

    Ok(Json(varticulos))
}

#[get("/profile/<user_id>")]
async fn profile(state: &State<AppState>, user_id: i32) -> Result<Json<Cliente>, Status> {
    let pool = state.pool.clone();

    let cliente = postgres_get_cliente_by_user_id(&pool, user_id)
        .await
        .map_err(|e| {
            eprintln!("Error getting client: {:?}", e);
            Status::InternalServerError
        })?;

    Ok(Json(cliente))
}
