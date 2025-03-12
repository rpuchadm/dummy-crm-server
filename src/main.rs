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
use sqlx::{Decode, FromRow};

mod articulos;
mod clientes;
mod postgresini;

use articulos::{Articulo, postgres_get_articulos};
use clientes::{Cliente, postgres_get_cliente_by_user_id};

struct AppState {
    pool: sqlx::Pool<sqlx::Postgres>,
}

#[launch]
async fn rocket() -> _ {
    let pool: sqlx::Pool<sqlx::Postgres> = sqlx::postgres::PgPool::connect(POSTGRES_SERVER)
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

// constante con el servidor de postgres
const POSTGRES_SERVER: &str = "postgresql://myuser:mypassword@localhost:5432/mydatabase";
