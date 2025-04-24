use reqwest::Client;
use rocket::http::Status;
use rocket::outcome::Outcome;
use rocket::request::{self, FromRequest, Request};
use rocket::response::status::NotFound;
use rocket::serde::json::Json;
use rocket::serde::{Deserialize, Serialize};
use rocket::{State, get, launch, post, routes};
use rocket::{catch, catchers, put};
use rocket_cors::{AllowedHeaders, AllowedOrigins, CorsOptions};
use std::collections::HashMap;
use std::env;

mod articulos;
mod clientes;
mod corpservice;
mod issuerequest;
mod issueservice;
mod postgresini;
mod sesion;

use articulos::{
    Articulo, ArticuloRequest, postgres_create_articulo, postgres_get_articulo_by_id,
    postgres_get_articulos, postgres_update_articulo,
};
use clientes::{Cliente, postgres_get_cliente_by_user_id};
use sesion::{AuthProfile, redis_get_session_by_token, redis_set_session_by_token};

struct AppState {
    pool: sqlx::Pool<sqlx::Postgres>,
    redis_connection_string: String,
    auth_redis_ttl: i64,
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

    let auth_redis_ttl = std::env::var("AUTH_REDIS_TTL")
        .unwrap_or_else(|_| "120".to_string())
        .parse::<i64>()
        .expect("AUTH_REDIS_TTL must be a number");

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
            auth_redis_ttl,
        })
        .mount(
            "/",
            routes![
                auth,
                authback,
                getarticulo,
                getarticulos,
                healthz,
                postarticulo,
                postissue,
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

#[derive(Clone)]
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
        /*.await
        .map_err(|e| {
            eprintln!("Error getting profile: {:?}", e);
            Status::InternalServerError
        })?;*/
        .await;

    match response {
        Ok(response) => {
            // Verificar el código de estado de la respuesta
            match response.status().as_u16() {
                200 => {
                    // Parsear la respuesta JSON a la estructura AuthProfile
                    let profile = response.json::<AuthProfile>().await.map_err(|e| {
                        eprintln!("Error parsing profile: {:?}", e);
                        Status::InternalServerError
                    })?;

                    Ok(Some(profile))
                }
                401 => {
                    eprintln!("auth_profile response status: 401 Unauthorized");
                    Err(Status::Unauthorized) // Devolver 401 Unauthorized
                }
                _ => {
                    eprintln!("auth_profile response status: {}", response.status());
                    Err(Status::InternalServerError) // Devolver 500 para otros errores
                }
            }
        }
        Err(e) => {
            eprintln!("Error getting profile: {:?}", e);
            let errmsg = e.to_string();
            if errmsg.contains("401") {
                // status: 401 Unauthorized
                return Err(Status::Unauthorized);
            }
            Err(Status::InternalServerError)
        }
    }
}

#[derive(Serialize, Deserialize)]
struct AuthResponse {
    status: String,
    user_id: i32,
    attributes: HashMap<String, String>,
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

    println!("profile con redis: {:?}", profile);

    let user_id = profile.as_ref().map(|p| p.user_id).unwrap_or(0);
    let attributes = profile
        .as_ref()
        .map(|p| p.attributes.clone())
        .unwrap_or(HashMap::new());

    if profile.is_some() {
        return Ok(Json(AuthResponse {
            status: "success".to_string(),
            user_id,
            attributes,
        }));
    }

    let profile = auth_profile(token).await?;

    if profile.is_none() {
        return Err(Status::Unauthorized);
    }

    let profile = profile.unwrap();

    println!("profile sin redis: {:?}", profile);

    if profile.user_id == 0 {
        return Err(Status::Forbidden);
    }

    let user_id = profile.user_id;
    let attributes = profile.attributes.clone();

    redis_set_session_by_token(&redis_client, &token_str, &profile, state.auth_redis_ttl)
        .await
        .map_err(|e| {
            eprintln!("Error setting session: {:?}", e);
            Status::InternalServerError
        })?;

    Ok(Json(AuthResponse {
        status: "success".to_string(),
        user_id,
        attributes,
    }))
}

#[get("/articulos")]
async fn getarticulos(
    state: &rocket::State<AppState>,
    token: BearerToken,
) -> Result<Json<Vec<Articulo>>, Status> {
    let profile = auth_profile(token).await?;

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

#[derive(Serialize, Deserialize)]
struct ArticuloData {
    articulo: Articulo,
    issue_requests: Vec<issuerequest::IssueRequest>,
}

#[get("/articulo/<id>")]
async fn getarticulo(
    state: &rocket::State<AppState>,
    token: BearerToken,
    id: i32,
) -> Result<Json<ArticuloData>, Status> {
    let profile = auth_profile(token).await?;

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

    let issue_requests = issuerequest::postgres_get_issue_requests_by_articulo(&pool, id)
        .await
        .map_err(|e| {
            eprintln!("Error getting issue requests: {:?}", e);
            Status::InternalServerError
        })?;
    let articulo = ArticuloData {
        articulo: articulo.clone(),
        issue_requests,
    };
    Ok(Json(articulo))
}

#[post("/articulo/<id>", data = "<articulo>")]
async fn postarticulo(
    state: &rocket::State<AppState>,
    token: BearerToken,
    articulo: Json<ArticuloRequest>,
    id: i32,
) -> Result<Json<Articulo>, Status> {
    let profile = auth_profile(token).await?;

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
    let profile = auth_profile(token).await?;

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

#[derive(Serialize, Deserialize)]
struct GetProfileResponse {
    cliente: Option<Cliente>,
    corp_user: Option<corpservice::UserData>,
    issue_requests: Vec<issuerequest::IssueRequest>,
}

#[get("/profile/<id>")]
async fn profile(
    state: &State<AppState>,
    token: BearerToken,
    id: i32,
) -> Result<Json<GetProfileResponse>, Status> {
    // Autenticación
    let profile = auth_profile(token).await?;
    let profile = profile.ok_or(Status::Unauthorized)?;

    // Autorización
    if profile.user_id == 0 {
        return Err(Status::Forbidden);
    }

    if id != profile.user_id {
        let default_role = "".to_string();
        let role = profile.attributes.get("role").unwrap_or(&default_role);
        if role != "admin" {
            return Err(Status::Forbidden);
        }
    }
    // Obtener datos
    let pool = state.pool.clone();

    let cliente = postgres_get_cliente_by_user_id(&pool, id)
        .await
        .map_err(|e| {
            eprintln!("Error getting client: {:?}", e);
            Status::InternalServerError
        })?;

    let corp_user = corpservice::corp_service_userdata_by_id(id)
        .await
        .map_err(|e| {
            eprintln!("Error fetching corp user data: {:?}", e);
            Status::FailedDependency // 424 - Dependencia fallida
        })?;

    let issue_requests = issuerequest::postgres_get_issue_requests_by_cliente(&pool, id)
        .await
        .map_err(|e| {
            eprintln!("Error getting issue requests: {:?}", e);
            Status::InternalServerError
        })?;

    let data = GetProfileResponse {
        cliente,
        corp_user,
        issue_requests,
    };

    Ok(Json(data))
}

#[get("/profiles")]
async fn profiles(
    state: &State<AppState>,
    token: BearerToken,
) -> Result<Json<Vec<Cliente>>, Status> {
    let profile = auth_profile(token).await?;

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
    let profile = auth_profile(token).await?;

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
    let profile = auth_profile(token).await?;

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

#[post("/issue/<tipo>/<id>", data = "<issuepostrequest>")]
async fn postissue(
    state: &rocket::State<AppState>,
    token: BearerToken,
    mut issuepostrequest: Json<issuerequest::IssuePostRequest>,
    tipo: &str,
    id: i32,
) -> Result<Json<issuerequest::IssueRequest>, Status> {
    let profile = auth_profile(token.clone()).await?;

    if profile.is_none() {
        return Err(Status::Unauthorized);
    }

    let profile = profile.unwrap();

    if profile.user_id == 0 {
        return Err(Status::Forbidden);
    }

    // si id es 0 da error
    if id == 0 {
        eprintln!("Error creating issue request: id must not be 0");
        return Err(Status::BadRequest);
    }
    // si tipo es vacio da error
    if tipo.is_empty() {
        eprintln!("Error creating issue request: type cannot be empty");
        return Err(Status::BadRequest);
    }
    // si tipo no es articulo o pedido da error
    if tipo != "articulo" && tipo != "cliente" && tipo != "pedido" {
        eprintln!("Error creating issue request: type must be articulo,cliente or pedido");
        return Err(Status::BadRequest);
    }

    // si subject o description son vacios da error
    if issuepostrequest.subject.is_empty() || issuepostrequest.description.is_empty() {
        eprintln!("Error creating issue request: subject or description cannot be empty");
        return Err(Status::BadRequest);
    }

    if tipo == "articulo" {
        issuepostrequest.subject = format!(
            "{}\nhttps://crm.mydomain.com/articulo/{}",
            issuepostrequest.subject, id
        );
    } else if tipo == "cliente" {
        issuepostrequest.subject = format!(
            "{}\nhttps://crm.mydomain.com/profile/{}",
            issuepostrequest.subject, id
        );
    } else if tipo == "pedido" {
        issuepostrequest.subject = format!(
            "{}\nhttps://crm.mydomain.com/pedido/{}",
            issuepostrequest.subject, id
        );
    }

    let pool = state.pool.clone();
    let issue_request = issuerequest::IssueRequest {
        id: 0,
        fecha_creacion: chrono::Utc::now().naive_utc(),
        data: serde_json::json!({
            "type": tipo,
            "id": id,
            "subject": issuepostrequest.subject,
            "description": issuepostrequest.description,
        }),
    };

    let new_issue_request = issuerequest::postgres_create_issue_request(&pool, issue_request)
        .await
        .map_err(|e| {
            eprintln!("Error creating issue request: {:?}", e);
            Status::InternalServerError
        })?;

    if new_issue_request.id == 0 {
        eprintln!("Error creating issue request: id is 0");
        return Err(Status::InternalServerError);
    }

    // si tipo es articulo o cliente o pedido crea la relacion
    if tipo == "articulo" {
        let _ =
            issuerequest::postgres_create_issue_request_articulo(&pool, new_issue_request.id, id)
                .await
                .map_err(|e| {
                    eprintln!("Error creating issue request article relation: {:?}", e);
                    Status::InternalServerError
                })?;
    } else if tipo == "cliente" {
        let _ =
            issuerequest::postgres_create_issue_request_cliente(&pool, new_issue_request.id, id)
                .await
                .map_err(|e| {
                    eprintln!("Error creating issue request client relation: {:?}", e);
                    Status::InternalServerError
                })?;
    } else if tipo == "pedido" {
        let _ = issuerequest::postgres_create_issue_request_pedido(&pool, new_issue_request.id, id)
            .await
            .map_err(|e| {
                eprintln!("Error creating issue request order relation: {:?}", e);
                Status::InternalServerError
            })?;
    }

    // TODO hacer peticion rest con el token, id_proyecto fijo, subject y description
    let issue_service_post_data = issueservice::IssueServicePostData {
        subject: issuepostrequest.subject.clone(),
        description: issuepostrequest.description.clone(),
        project_id: 0,
        tracker_id: 0,
    };

    let issue_service_post_ret =
        issueservice::issue_service_post(issue_service_post_data, token.0.clone())
            .await
            .map_err(|e| {
                eprintln!("Error creating issue in issue service: {:?}", e);
                Status::InternalServerError
            })?;
    if issue_service_post_ret.is_empty() {
        eprintln!("Error creating issue in issue service: empty response");
        return Err(Status::InternalServerError);
    }
    // si issue_service_post_ret no es un numero da error
    if issue_service_post_ret.parse::<i32>().is_err() {
        eprintln!("Error creating issue in issue service: invalid response");
        return Err(Status::InternalServerError);
    }
    // si issue_service_post_ret es 0 da error
    if issue_service_post_ret.parse::<i32>().unwrap() == 0 {
        eprintln!("Error creating issue in issue service: id is 0");
        return Err(Status::InternalServerError);
    }

    Ok(Json(new_issue_request))
}
