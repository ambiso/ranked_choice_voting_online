use axum::response::Html;

use sqlx::PgPool;

use crate::config::Config;

use sailfish::TemplateSimple;
use std::sync::Arc;
mod create_election;
mod election;
mod add_candidate;
mod vote;
pub(crate) use create_election::create;
pub(crate) use election::election;
pub(crate) use add_candidate::add_candidate;
pub(crate) use vote::vote;

#[derive(TemplateSimple)]
#[template(path = "index.html.stpl")]
pub(crate) struct IndexTemplate {}

pub(crate) async fn index() -> Html<String> {
    Html(IndexTemplate {}.render_once().unwrap())
}

#[derive(Clone)]
pub(crate) struct ApiContext {
    pub(crate) config: Arc<Config>,
    pub(crate) db: PgPool,
}
