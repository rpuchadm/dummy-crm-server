use std::collections::HashMap;

use redis::AsyncCommands;
use rocket::serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct AuthProfile {
    pub id: i32,
    pub client_id: String,
    pub user_id: i32,
    pub attributes: HashMap<String, String>,
}

const SESSION_TOKEN_KEY: &str = "session-token:";
const SESSION_TIME_SECONDS: i64 = 60 * 2; // 2 minutos
pub async fn redis_get_session_by_token(
    client: &redis::Client,
    token: &str,
) -> redis::RedisResult<Option<AuthProfile>> {
    let key = format!("{}:{}", SESSION_TOKEN_KEY, token);

    let mut con = client.get_multiplexed_async_connection().await.unwrap();
    let session_json: Option<String> = con.get(&key).await?;
    let session: Option<AuthProfile> = match session_json {
        Some(session_json) => Some(serde_json::from_str(&session_json).unwrap()),
        None => None,
    };
    Ok(session)
}
pub async fn redis_set_session_by_token(
    client: &redis::Client,
    token: &str,
    session: &AuthProfile,
) -> redis::RedisResult<()> {
    let key = format!("{}:{}", SESSION_TOKEN_KEY, token);
    let mut con = client.get_multiplexed_async_connection().await.unwrap();
    let session_json = serde_json::to_string(session).unwrap();
    let _: () = con.set(&key, session_json).await?;
    let _: () = con.expire(&key, SESSION_TIME_SECONDS).await?;
    Ok(())
}
