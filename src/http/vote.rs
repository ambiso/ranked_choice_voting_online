use std::collections::{HashMap, HashSet};

use crate::error::{Error, Result};
use axum::response::Html;
use axum::{extract::Path, response::Redirect};
use sailfish::TemplateSimple;
use tracing::debug;
use uuid::Uuid;

use super::ApiContext;

use axum::{Extension, Form};

struct Candidate {
    id: i64,
}

pub(crate) async fn vote(
    ctx: Extension<ApiContext>,
    Path(election_id): Path<Uuid>,
    Form(vote_data): Form<HashMap<String, String>>,
) -> Result<Redirect> {
    let mut prefs = Vec::new();
    for (k, v) in &vote_data {
        if v.len() == 0 {
            continue;
        }
        let rank: u8 = k
            .strip_prefix("opt")
            .ok_or(Error::UserInputWrong)?
            .parse()
            .map_err(|_| Error::UserInputWrong)?;
        if prefs.len() <= rank as usize {
            prefs.resize(rank as usize + 1, None);
        }
        prefs[rank as usize] = Some(v.parse().map_err(|_| Error::UserInputWrong)?);
    }
    if prefs.iter().any(Option::is_none) {
        debug!("Voting preference has holes");
        return Err(Error::UserInputWrong);
    }

    if prefs
        .iter()
        .map(|x| x.unwrap())
        .collect::<HashSet<_>>()
        .len()
        != prefs.len()
    {
        debug!("Voting preference has duplicates");
        return Err(Error::UserInputWrong);
    }

    let vote_id = Uuid::new_v4();
    let candidate_ids: Vec<_> = prefs.iter().map(|x| x.unwrap()).collect();
    let preferences: Vec<_> = (0..prefs.len() as i32).collect();
    let mut tx = ctx.db.begin().await?;
    sqlx::query!(
        "insert into vote(id, election) values ($1, $2)",
        vote_id,
        election_id,
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!(
        "insert into vote_preferences(vote, candidate, preference) 
        SELECT * FROM UNNEST($1::uuid[], $2::bigint[], $3::int[])",
        &vec![vote_id; prefs.len()],
        &candidate_ids[..],
        &preferences[..],
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Redirect::to(&format!("/election/{election_id}")))
}
