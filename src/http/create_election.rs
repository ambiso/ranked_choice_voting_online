use crate::error::Result;
use axum::response::Redirect;
use serde::Deserialize;

use super::ApiContext;

use axum::Extension;

use axum::Form;

#[derive(Deserialize)]
pub(crate) struct CreateData {
    pub(crate) title: String,
}

pub(crate) async fn create(
    ctx: Extension<ApiContext>,
    Form(create_data): Form<CreateData>,
) -> Result<Redirect> {
    let mut tx = ctx.db.begin().await?;
    let uuid = sqlx::types::Uuid::new_v4();
    sqlx::query!(
        "insert into election(id, title) values ($1, $2)",
        uuid,
        create_data.title,
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Redirect::to(&format!("/election/{uuid}")))
}
