use serde::Serialize;
use sqlx::{Decode, FromRow};

#[derive(Serialize, Clone, FromRow)]
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
