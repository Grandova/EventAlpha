use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub username: String,
    pub created_at: i64,
    pub expires_at: i64,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub success: bool,
    pub token: Option<String>,
    pub username: Option<String>,
    pub expires_at: Option<i64>,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct AuthStatusResponse {
    pub authenticated: bool,
    pub auth_enabled: bool,
    pub username: Option<String>,
}

pub async fn handle_login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> (StatusCode, Json<LoginResponse>) {
    if !state.config.auth.enabled {
        let token = format!("pqt_{}", Uuid::new_v4().simple());
        let expires_at = Utc::now().timestamp_millis() + 86400 * 1000 * 30;
        state.sessions.insert(
            token.clone(),
            SessionInfo {
                username: payload.username.clone(),
                created_at: Utc::now().timestamp_millis(),
                expires_at,
            },
        );
        return (
            StatusCode::OK,
            Json(LoginResponse {
                success: true,
                token: Some(token),
                username: Some(payload.username),
                expires_at: Some(expires_at),
                message: "登录成功 (免密模式)".to_string(),
            }),
        );
    }

    if payload.username.trim() == state.config.auth.username
        && payload.password == state.config.auth.password
    {
        let token = format!("pqt_{}_{}", Uuid::new_v4().simple(), Utc::now().timestamp_millis());
        let expires_at = Utc::now().timestamp_millis()
            + (state.config.auth.session_timeout_hours as i64) * 3600 * 1000;

        state.sessions.insert(
            token.clone(),
            SessionInfo {
                username: payload.username.clone(),
                created_at: Utc::now().timestamp_millis(),
                expires_at,
            },
        );

        tracing::info!("User '{}' logged in successfully.", payload.username);
        (
            StatusCode::OK,
            Json(LoginResponse {
                success: true,
                token: Some(token),
                username: Some(payload.username),
                expires_at: Some(expires_at),
                message: "登录成功".to_string(),
            }),
        )
    } else {
        tracing::warn!("Failed login attempt for username '{}'.", payload.username);
        (
            StatusCode::UNAUTHORIZED,
            Json(LoginResponse {
                success: false,
                token: None,
                username: None,
                expires_at: None,
                message: "用户名或密码错误，请检查".to_string(),
            }),
        )
    }
}

pub async fn handle_auth_me(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> (StatusCode, Json<AuthStatusResponse>) {
    if !state.config.auth.enabled {
        return (
            StatusCode::OK,
            Json(AuthStatusResponse {
                authenticated: true,
                auth_enabled: false,
                username: Some("admin".to_string()),
            }),
        );
    }

    let auth_header = headers.get("authorization").and_then(|h| h.to_str().ok());
    if let Some(token) = extract_bearer_token(auth_header) {
        if let Some(session) = state.sessions.get(token) {
            let now = Utc::now().timestamp_millis();
            if session.expires_at > now {
                return (
                    StatusCode::OK,
                    Json(AuthStatusResponse {
                        authenticated: true,
                        auth_enabled: true,
                        username: Some(session.username.clone()),
                    }),
                );
            }
        }
    }

    (
        StatusCode::UNAUTHORIZED,
        Json(AuthStatusResponse {
            authenticated: false,
            auth_enabled: true,
            username: None,
        }),
    )
}

pub async fn handle_logout(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> (StatusCode, Json<serde_json::Value>) {
    let auth_header = headers.get("authorization").and_then(|h| h.to_str().ok());
    if let Some(token) = extract_bearer_token(auth_header) {
        state.sessions.remove(token);
    }
    (StatusCode::OK, Json(serde_json::json!({ "success": true, "message": "已安全退出登录" })))
}

pub fn extract_bearer_token(header_val: Option<&str>) -> Option<&str> {
    header_val.and_then(|h| {
        let trimmed = h.trim();
        if trimmed.starts_with("Bearer ") {
            Some(trimmed["Bearer ".len()..].trim())
        } else {
            None
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_bearer_token() {
        assert_eq!(extract_bearer_token(Some("Bearer abc123xyz")), Some("abc123xyz"));
        assert_eq!(extract_bearer_token(Some("Bearer   my_token   ")), Some("my_token"));
        assert_eq!(extract_bearer_token(Some("Basic abc")), None);
        assert_eq!(extract_bearer_token(None), None);
    }
}
