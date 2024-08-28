use crate::error::Result;
use axum::extract::Path;
use axum::response::Redirect;
use serde::Deserialize;
use uuid::Uuid;

use super::ApiContext;

use axum::Extension;

use axum::Form;

#[derive(Deserialize)]
pub(crate) struct CreateData {
    pub(crate) username: String,
}

pub(crate) async fn add_candidate(
    ctx: Extension<ApiContext>,
    Path(election_id): Path<Uuid>,
    Form(create_data): Form<CreateData>,
) -> Result<Redirect> {
    let mut tx = ctx.db.begin().await?;
    sqlx::query!(
        "insert into candidate(election, username) values ($1, $2)",
        election_id,
        create_data.username,
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Redirect::to(&format!("/election/{}", election_id)))
}
