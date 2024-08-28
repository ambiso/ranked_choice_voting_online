use std::{
    collections::{BTreeMap, HashMap, HashSet},
    hash::Hash,
};

use crate::error::{Error, Result};
use axum::extract::Path;
use axum::response::Html;
use sailfish::TemplateSimple;
use tokio::sync::mpsc::error;
use tracing::{debug, error};
use uuid::Uuid;

use super::ApiContext;

use axum::Extension;

#[derive(TemplateSimple)]
#[template(path = "election.html.stpl")]
pub(crate) struct ElectionTemplate<'a, 'b, 'c, 'd, 'e> {
    title: &'a str,
    candidates: &'c [Candidate],
    election_id: String,
    instant_runoff: &'b str,
    instant_runoff_tallies: Vec<Vec<f64>>,
    voters: BTreeMap<Uuid, Voter>,
    candidates_map: HashMap<i64, &'d str>,
    condorcet_tally: Vec<f64>,
    condorcet_winner: &'e str,
}

struct Candidate {
    username: String,
    id: i64,
}

struct Preference {
    vote: Uuid,
    candidate: i64,
    preference: i32,
}

#[derive(Default, Debug)]
struct Voter {
    preferences: Vec<Option<i64>>,
    rank: usize,
}

pub(crate) async fn election(
    ctx: Extension<ApiContext>,
    Path(election_id): Path<Uuid>,
) -> Result<Html<String>> {
    let mut tx = ctx.db.begin().await?;
    let row = sqlx::query!("select title from election where id = $1", election_id,)
        .fetch_one(&mut *tx)
        .await?;

    let candidates = sqlx::query_as!(
        Candidate,
        "select username, id from candidate where election = $1",
        election_id
    )
    .fetch_all(&mut *tx)
    .await?;

    let candidates_map: HashMap<_, _> = candidates
        .iter()
        .map(|c| (c.id, c.username.as_str()))
        .collect();

    let votes = sqlx::query_as!(Preference, "select vote, candidate, preference from vote_preferences join vote on vote_preferences.vote = vote.id where election = $1;", election_id).fetch_all(&mut *tx).await?;

    // compute Condorcet winner

    let mut voters = BTreeMap::<_, Voter>::new();
    let mut vote_candidates = HashSet::new();

    for vote in &votes {
        let voter: &mut Voter = voters.entry(vote.vote).or_default();
        let prefs = &mut voter.preferences;
        if prefs.len() <= vote.preference as usize {
            if vote.preference > 256 {
                error!("Too many candidates?");
                return Err(Error::UserInputWrong);
            }
            prefs.resize(vote.preference as usize + 1, None);
        }
        prefs[vote.preference as usize] = Some(vote.candidate);
        vote_candidates.insert(vote.candidate);
    }

    for voter in voters.values() {
        if voter.preferences.iter().any(Option::is_none) {
            error!("Voter preferences has holes");
            return Err(Error::UserInputWrong);
        }
    }

    // compute instant runoff winner

    let mut instant_runoff = None;
    let mut tallies = Vec::<Vec<f64>>::new();
    let mut eliminated = HashSet::new();
    let candidate_index: HashMap<_, _> = candidates
        .iter()
        .enumerate()
        .map(|(i, c)| (c.id, i))
        .collect();
    'outer: loop {
        let mut tally = HashMap::<_, u64>::new();
        let mut total = 0;
        for voter in voters.values() {
            let Some(candidate) = voter.preferences.get(voter.rank) else {
                continue;
            };
            *tally.entry(candidate.unwrap()).or_default() += 1;
            total += 1;
        }
        let mut ordered_tally = vec![0.0; candidates.len()];
        for (candidate, count) in tally.iter() {
            ordered_tally[candidate_index[candidate]] = *count as f64 / total as f64 * 100.0;
        }
        tallies.push(ordered_tally);
        for (&candidate, &count) in &tally {
            if count > total / 2 {
                instant_runoff = Some(candidate);
                break 'outer;
            }
        }
        let Some((_, &eliminate_candidate)) = tally
            .iter()
            .map(|(candidate, votes)| (votes, candidate))
            .min()
        else {
            break;
        };
        debug!(
            "Eliminating {eliminate_candidate} ({})",
            candidates_map
                .get(&eliminate_candidate)
                .map(|s| *s)
                .unwrap_or("(Not found)")
        );
        eliminated.insert(eliminate_candidate);

        for voter in voters.values_mut() {
            while voter
                .preferences
                .get(voter.rank)
                .map(|x| eliminated.contains(&x.unwrap()))
                .unwrap_or(false)
            {
                voter.rank += 1;
            }
        }
    }

    let mut condorcet_tally = vec![0.0; candidates.len()];
    let mut condorcet_winner = None;
    for ca in candidates.iter() {
        let mut all = true;
        let mut a_wins = 0;
        for cb in candidates.iter() {
            if ca.id == cb.id {
                continue;
            }

            let mut a_tally = 0;
            let mut b_tally = 0;
            for (_, voter) in voters.iter() {
                let a_idx = voter.preferences.iter().position(|x| x.unwrap() == ca.id);
                let b_idx = voter.preferences.iter().position(|x| x.unwrap() == cb.id);
                match (a_idx, b_idx) {
                    (Some(a), Some(b)) => {
                        if a < b {
                            a_tally += 1;
                        } else if b < a {
                            b_tally += 1;
                        } else {
                            unreachable!()
                        }
                    }
                    (Some(_), _) => a_tally += 1,
                    (_, Some(_)) => b_tally += 1,
                    (None, None) => {}
                }
            }
            if a_tally > b_tally {
                a_wins += 1;
            } else {
                all = false;
            }
        }
        condorcet_tally[candidate_index[&ca.id]] =
            a_wins as f64 / (candidates.len() - 1) as f64 * 100.0;
        if all {
            condorcet_winner = Some(ca.username.as_str());
        }
    }

    Ok(Html(
        ElectionTemplate {
            title: &row.title,
            candidates: &candidates,
            election_id: election_id.to_string(),
            instant_runoff: instant_runoff
                .map(|x| candidates_map.get(&x).map(|x| *x).unwrap_or("(Not found)"))
                .unwrap_or("No winner"),
            instant_runoff_tallies: tallies,
            voters,
            candidates_map,
            condorcet_tally,
            condorcet_winner: condorcet_winner.unwrap_or("No winner"),
        }
        .render_once()?,
    ))
}
