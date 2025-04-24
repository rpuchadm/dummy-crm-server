use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Serialize, Deserialize, Clone, Debug, FromRow)]
pub struct IssueRequest {
    pub id: i32,
    pub fecha_creacion: chrono::NaiveDateTime,
    pub data: serde_json::Value,
}

pub async fn postgres_get_issue_requests_by_articulo(
    pool: &sqlx::Pool<sqlx::Postgres>,
    articulo_id: i32,
) -> Result<Vec<IssueRequest>, sqlx::Error> {
    let issue_requests = sqlx::query_as::<_, IssueRequest>(
        "
        SELECT
            id, fecha_creacion, data
        FROM issue_request
        where
            id in (
                SELECT
                    issue_request_id
                FROM issue_request_articulos
                WHERE articulo_id = $1
            )
        ORDER BY fecha_creacion DESC",
    )
    .bind(articulo_id)
    .fetch_all(pool)
    .await?;

    Ok(issue_requests)
}

pub async fn postgres_get_issue_requests_by_cliente(
    pool: &sqlx::Pool<sqlx::Postgres>,
    cliente_id: i32,
) -> Result<Vec<IssueRequest>, sqlx::Error> {
    let issue_requests = sqlx::query_as::<_, IssueRequest>(
        "
        SELECT
            id, fecha_creacion, data
        FROM issue_request
        where
            id in (
                SELECT
                    issue_request_id
                FROM issue_request_clientes
                WHERE cliente_id = $1
            )
        ORDER BY fecha_creacion DESC",
    )
    .bind(cliente_id)
    .fetch_all(pool)
    .await?;

    Ok(issue_requests)
}

/*
pub async fn postgres_get_issue_requests_by_pedido(
    pool: &sqlx::Pool<sqlx::Postgres>,
    pedido_id: i32,
) -> Result<Vec<IssueRequest>, sqlx::Error> {
    let issue_requests = sqlx::query_as::<_, IssueRequest>(
        "
        SELECT
            id, fecha_creacion, data
        FROM issue_request
        where
            id in (
                SELECT
                    issue_request_id
                FROM issue_request_pedidos
                WHERE pedido_id = $1
            )
        ORDER BY fecha_creacion DESC",
    )
    .bind(pedido_id)
    .fetch_all(pool)
    .await?;

    Ok(issue_requests)
}
*/

pub async fn postgres_create_issue_request(
    pool: &sqlx::Pool<sqlx::Postgres>,
    issue_request: IssueRequest,
) -> Result<IssueRequest, sqlx::Error> {
    let new_issue_request = sqlx::query_as::<_, IssueRequest>(
        "
        INSERT INTO issue_request (data)
        VALUES ($1)
        RETURNING id, fecha_creacion, data",
    )
    .bind(issue_request.data)
    .fetch_one(pool)
    .await?;

    Ok(new_issue_request)
}

pub async fn postgres_create_issue_request_articulo(
    pool: &sqlx::Pool<sqlx::Postgres>,
    issue_request_id: i32,
    articulo_id: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "
        INSERT INTO issue_request_articulos (issue_request_id, articulo_id)
        VALUES ($1, $2)",
    )
    .bind(issue_request_id)
    .bind(articulo_id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn postgres_create_issue_request_cliente(
    pool: &sqlx::Pool<sqlx::Postgres>,
    issue_request_id: i32,
    cliente_id: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "
        INSERT INTO issue_request_clientes (issue_request_id, cliente_id)
        VALUES ($1, $2)",
    )
    .bind(issue_request_id)
    .bind(cliente_id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn postgres_create_issue_request_pedido(
    pool: &sqlx::Pool<sqlx::Postgres>,
    issue_request_id: i32,
    pedido_id: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "
        INSERT INTO issue_request_pedidos (issue_request_id, pedido_id)
        VALUES ($1, $2)",
    )
    .bind(issue_request_id)
    .bind(pedido_id)
    .execute(pool)
    .await?;

    Ok(())
}
