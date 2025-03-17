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

pub async fn postgres_get_cliente_by_id(
    pool: &sqlx::Pool<sqlx::Postgres>,
    user_id: i32,
) -> Result<Cliente, sqlx::Error> {
    let cliente: Cliente = sqlx::query_as::<_, Cliente>(
        "SELECT
            id, user_id, nombre, email, telefono, direccion, fecha_registro
        FROM clientes
        WHERE id = $1",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    Ok(cliente)
}

pub async fn postgres_get_cliente_by_user_id(
    pool: &sqlx::Pool<sqlx::Postgres>,
    user_id: i32,
) -> Result<Cliente, sqlx::Error> {
    let cliente: Cliente = sqlx::query_as::<_, Cliente>(
        "SELECT
            id, user_id, nombre, email, telefono, direccion, fecha_registro
        FROM clientes
        WHERE user_id = $1",
    )
    .bind(user_id)
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
