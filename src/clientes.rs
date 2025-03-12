use serde::Serialize;
use sqlx::{Decode, FromRow};

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
