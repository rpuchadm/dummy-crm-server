use serde::{Deserialize, Serialize};
use sqlx::{Decode, FromRow};

#[derive(Serialize, Deserialize, Clone, FromRow, Decode, Debug)]
pub struct ArticuloRequest {
    pub id: i32,
    pub nombre: String,
    pub descripcion: Option<String>,
    pub precio: i32,
    pub stock: i32,
}

#[derive(Serialize, Deserialize, Clone, FromRow, Decode)]
pub struct Articulo {
    pub id: i32,
    pub nombre: String,
    pub descripcion: Option<String>,
    pub precio: i32,
    pub stock: i32,
    pub fecha_creacion: chrono::NaiveDateTime,
}

pub async fn postgres_get_articulos(
    pool: &sqlx::Pool<sqlx::Postgres>,
) -> Result<Vec<Articulo>, sqlx::Error> {
    let articulos = sqlx::query_as::<_, Articulo>(
        "
        SELECT
            id, nombre, descripcion, precio, stock, fecha_creacion
        FROM articulos",
    )
    .fetch_all(pool)
    .await?;

    Ok(articulos)
}

pub async fn postgres_get_articulo_by_id(
    pool: &sqlx::Pool<sqlx::Postgres>,
    id: i32,
) -> Result<Articulo, sqlx::Error> {
    let articulo = sqlx::query_as::<_, Articulo>(
        "
        SELECT
            id, nombre, descripcion, precio, stock, fecha_creacion
        FROM articulos
        WHERE id = $1",
    )
    .bind(id)
    .fetch_one(pool)
    .await?;

    Ok(articulo)
}

pub async fn postgres_create_articulo(
    pool: &sqlx::Pool<sqlx::Postgres>,
    articulo: ArticuloRequest,
) -> Result<Articulo, sqlx::Error> {
    let new_articulo = sqlx::query_as::<_, Articulo>(
        r#"
        INSERT INTO articulos (nombre, descripcion, precio, stock, fecha_creacion)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, nombre, descripcion, precio, stock, fecha_creacion
        "#,
    )
    .bind(articulo.nombre)
    .bind(articulo.descripcion)
    .bind(articulo.precio)
    .bind(articulo.stock)
    .bind(chrono::Utc::now().naive_utc())
    .fetch_one(pool)
    .await?;

    Ok(new_articulo)
}

pub async fn postgres_update_articulo(
    pool: &sqlx::Pool<sqlx::Postgres>,
    articulo: ArticuloRequest,
    id: i32,
) -> Result<Articulo, sqlx::Error> {
    let updated_articulo = sqlx::query_as::<_, Articulo>(
        r#"
        UPDATE articulos
        SET nombre = $1, descripcion = $2, precio = $3, stock = $4
        WHERE id = $5
        RETURNING id, nombre, descripcion, precio, stock, fecha_creacion
        "#,
    )
    .bind(articulo.nombre)
    .bind(articulo.descripcion)
    .bind(articulo.precio)
    .bind(articulo.stock)
    .bind(id)
    .fetch_one(pool)
    .await?;

    Ok(updated_articulo)
}
