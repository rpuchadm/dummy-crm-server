use chrono::prelude::*;
use redis::AsyncCommands;
use rocket::form::Form;
use rocket::http::Status;
use rocket::outcome::Outcome;
use rocket::request::{self, FromRequest, Request};
use rocket::response::status::NotFound;
use rocket::serde::json::Json;
use rocket::serde::{Deserialize, Serialize};
use rocket::{FromForm, catch, catchers, put};
use rocket::{State, delete, get, launch, post, routes};
use rocket_cors::{AllowedHeaders, AllowedOrigins, CorsOptions};
// put
use sqlx::{Decode, FromRow, postgres};
use std::env;

mod articulos;
mod clientes;
mod postgresini;

use articulos::{
    Articulo, ArticuloRequest, postgres_create_articulo, postgres_get_articulo_by_id,
    postgres_get_articulos, postgres_update_articulo,
};
use clientes::{Cliente, postgres_get_cliente_by_id};

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
        .manage(AppState { pool })
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

const SUPER_SECRET: &str = "super_secret_token111";

#[derive(Serialize, Deserialize)]
struct AuthResponse {
    status: String,
}

#[get("/auth")]
async fn auth(token: BearerToken) -> Result<Json<AuthResponse>, Status> {
    if token.0 != SUPER_SECRET {
        eprintln!("Error invalid super secret token");
        return Err(Status::Unauthorized);
    }
    Ok(Json(AuthResponse {
        status: "success".to_string(),
    }))
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

#[get("/articulo/<id>")]
async fn getarticulo(state: &rocket::State<AppState>, id: i32) -> Result<Json<Articulo>, Status> {
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
    articulo: Json<ArticuloRequest>,
    id: i32,
) -> Result<Json<Articulo>, Status> {
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
    articulo: Json<ArticuloRequest>,
    id: i32,
) -> Result<Json<Articulo>, Status> {
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
async fn profile(state: &State<AppState>, id: i32) -> Result<Json<Cliente>, Status> {
    let pool = state.pool.clone();

    let cliente = postgres_get_cliente_by_id(&pool, id).await.map_err(|e| {
        eprintln!("Error getting client: {:?}", e);
        Status::InternalServerError
    })?;

    Ok(Json(cliente))
}

#[get("/profiles")]
async fn profiles(state: &State<AppState>) -> Result<Json<Vec<Cliente>>, Status> {
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
    cliente: Json<clientes::ClienteRequest>,
) -> Result<Json<Cliente>, Status> {
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
    cliente: Json<clientes::ClienteRequest>,
    user_id: i32,
) -> Result<Json<Cliente>, Status> {
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
