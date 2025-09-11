//! User session management and persistence.

use anyhow::anyhow;
use chrono::TimeDelta;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tracing::{debug, error, warn};
use uuid::Uuid;

use crate::{
    db::{self, users::User},
    error::AppError,
};

/// Token representing a user's session.
///
/// This struct is converted to a JWT and signed, then used
/// as the session cookie. It contains no "claims" on its own
/// and is only a signed identifier with which to look up the
/// session on the backend.
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct SessionToken(pub Uuid);

/// User session containing authentication state and messages.
#[derive(Debug, Deserialize, Serialize)]
pub struct Session {
    sid: SessionToken,
}

impl Session {
    /// Returns a clone of the session token.
    pub fn session_token(&self) -> SessionToken {
        self.sid
    }
}

/// Creates a new session for a user.
///
/// This function:
/// 1. Checks if the user is revoked
/// 2. Cleans expired sessions from the database
/// 3. Creates a new session with a 60-minute expiration
///
/// # Errors
///
/// Returns `AppError::unauthorized` if the user is revoked.
/// Returns database errors if session creation fails.
pub async fn make_user_session(pool: &SqlitePool, user: &User) -> Result<Session, AppError> {
    if user.is_revoked {
        return Err(AppError::unauthorized(anyhow::anyhow!("User is revoked")));
    }

    if let Err(err) = clean_expired_sessions(pool).await {
        // Log it, but don't prevent making a new session
        error!(error = ?err, "Could not clean expired user sessions.");
    }

    let now = chrono::Utc::now();
    let expires_at = now.checked_add_signed(DEFAULT_SESSION_DURATION).unwrap().timestamp();
    let record = sqlx::query!(
        r#"
            insert into user_sessions (
                user_id,
                expires_at
            )
            values (?, ?)
            returning token_id as "token_id: Uuid"
        "#,
        user.user_id,
        expires_at
    )
    .fetch_one(pool)
    .await?;

    debug!(username = user.username, "Created new user session.");
    Ok(Session {
        sid: SessionToken(record.token_id),
    })
}

/// Removes a session from the database.
///
/// Used for logout functionality.
///
/// # Errors
///
/// Returns database errors if deletion fails.
pub async fn remove_session(pool: &SqlitePool, session_token: &SessionToken) -> Result<(), AppError> {
    let res = sqlx::query!(
        r#"
                delete from user_sessions
                where token_id = ?
            "#,
        session_token.0
    )
    .execute(pool)
    .await?;

    let rows = res.rows_affected();
    debug!(rows, "Removed user session.");
    Ok(())
}

/// Result of a session lookup operation.
///
/// Contains the user, their session, and the signed token.
pub struct SessionLookup {
    pub user: User,
    pub signed_token: String,
}

/// Looks up a session and associated user from a session token.
///
/// # Errors
///
/// Returns `AppError::unauthorized` if the session doesn't exist.
/// Returns database errors if queries fail.
pub async fn from_token(pool: &SqlitePool, session_token: SessionToken, signed_token: String) -> Result<SessionLookup, AppError> {
    let mut tx = pool.begin().await?;
    let now = chrono::Utc::now();
    let new_expires = now
        .checked_add_signed(DEFAULT_SESSION_DURATION)
        .expect("It's not the year 2000...")
        .timestamp();
    let record = sqlx::query!(
        r#"
        update user_sessions
        set expires_at = $1
        where token_id = $2
        returning
            user_id as "user_id: Uuid",
            expires_at
        "#,
        new_expires,
        session_token.0,
    )
    .fetch_optional(&mut *tx)
    .await?;

    let Some(record) = record else {
        return Err(AppError::unauthorized(anyhow!("No user session found.")));
    };

    if record.expires_at < now.timestamp() {
        let _ignore = sqlx::query!(
            r#"
                delete from user_sessions
                where token_id = $1
            "#,
            session_token.0
        )
        .execute(&mut *tx)
        .await
        .inspect_err(|e| warn!(error = ?e, "Failed to delete expired session, continuing."));

        return Err(AppError::unauthorized(anyhow!("User session expired")));
    }

    let user = db::users::get_by_id(pool, record.user_id).await?;
    tx.commit().await?;
    Ok(SessionLookup { user, signed_token })
}

pub const DEFAULT_SESSION_MINUTES: i64 = 60;
const DEFAULT_SESSION_DURATION: TimeDelta = TimeDelta::minutes(DEFAULT_SESSION_MINUTES);

/// Removes expired sessions from the database.
///
/// Called automatically when creating new sessions to prevent
/// accumulation of expired session records.
///
/// # Errors
///
/// Returns database errors if deletion fails.
async fn clean_expired_sessions(pool: &SqlitePool) -> Result<(), AppError> {
    let now = chrono::Utc::now().timestamp();
    let res = sqlx::query!(
        r#"
            delete
            from user_sessions
            where expires_at < ?
        "#,
        now
    )
    .execute(pool)
    .await?;

    debug!(sessions_deleted = res.rows_affected(), "Deleted expired user sessions.");

    Ok(())
}
