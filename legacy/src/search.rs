use std::collections::HashSet;

use sqlx::postgres::PgPool;
use unidecode::unidecode;

use crate::error::{SiteReportResult, IntoReport};
use crate::robots::RobotPreview;

const MAX_ROBOTS: i32 = 48;

pub(crate) async fn search(db_pool: &PgPool, query: &str) -> SiteReportResult<Vec<RobotPreview>> {
    let query_terms = match to_query_terms(query) {
        Some(query_terms) => query_terms,
        None => return Ok(Vec::new()),
    };

    // Vector for storing the robots found by the search
    let mut found_robots = Vec::new();

    // We only want to show each robot once, so keep track of the ids
    let mut found_ids = HashSet::new();

    let query_numbers = to_query_numbers(&query_terms);

    if !query_numbers.is_empty() {
        let number_matches = search_by_number(db_pool, &query_numbers, MAX_ROBOTS)
            .await
            .map_err(|err| err.into_report(format!("failed to search by numbers {:?}", query_numbers)))?;

            for robot in number_matches {
                found_ids.insert(robot.id);
                found_robots.push(robot);
            }
    }

    let ident_matches = search_by_ident(db_pool, &query_terms, MAX_ROBOTS - found_robots.len() as i32)
        .await
        .map_err(|err| err.into_report(format!("failed search by idents {:?}", query_terms)))?;

    for robot in ident_matches {
        if !found_ids.contains(&robot.id) {
            found_ids.insert(robot.id);
            found_robots.push(robot);
        }
    }

    let full_text_matches = search_by_full_text(db_pool, query, MAX_ROBOTS - found_robots.len() as i32)
        .await
        .map_err(|err| err.into_report(format!("failed to search by full text {:?}", query)))?;

    for robot in full_text_matches {
        if !found_ids.contains(&robot.id) {
            found_ids.insert(robot.id);
            found_robots.push(robot);
        }
    }

    Ok(found_robots)
}

fn to_query_terms(query: &str) -> Option<Vec<String>> {
    // Split the query by whitespace and convert to lowercase ASCII
    let words = query
        .split_whitespace()
        .filter_map(|word| {
            // Apply the same transformation to the word as the transformation that Smolbotbot
            // applies to robot name prefixes to generate the ident: convert to lowercase ASCII
            // then remove all non-alphanumeric characters
            let mut word_lower_ascii = unidecode(word).to_lowercase();
            word_lower_ascii.retain(|char| char.is_ascii_alphanumeric());

            // Discard words which do not have any alphanumeric characters
            if word_lower_ascii.is_empty() {
                None
            } else {
                Some(word_lower_ascii)
            }
        })
        .collect::<Vec<_>>();

    if words.is_empty() {
        return None;
    }

    let mut query_terms = Vec::new();

    for word in words {
        // Create a copy of any words ending with "bot", with the "bot" removed
        if let Some(trimmed_word) = word.strip_suffix("bot").or(word.strip_suffix("bots")) {
            if !trimmed_word.is_empty() {
                query_terms.push(trimmed_word.to_owned());
            }
        }

        query_terms.push(word);
    }

    Some(query_terms)
}

fn to_query_numbers(query_terms: &[String]) -> Vec<i32> {
    query_terms
        .iter()
        .filter_map(|term| term.parse::<i32>().ok())
        .collect::<Vec<_>>()
}

async fn search_by_number(
    db_pool: &PgPool,
    query_numbers: &[i32],
    limit: i32,
) -> sqlx::Result<Vec<RobotPreview>>
{
    sqlx::query_as(
        "SELECT \
            id, robot_number, ident, prefix, suffix, plural, content_warning, image_thumb_path, \
            alt, custom_alt \
        FROM robots \
        WHERE robot_number = ANY($1) \
        LIMIT $2"
    )
    .bind(&query_numbers)
    .bind(limit)
    .fetch_all(db_pool)
    .await
}

async fn search_by_ident(
    db_pool: &PgPool,
    query_terms: &[String],
    limit: i32,
) -> sqlx::Result<Vec<RobotPreview>>
{
    sqlx::query_as(
        "SELECT \
            id, robot_number, ident, prefix, suffix, plural, content_warning, image_thumb_path, \
            alt, custom_alt \
        FROM robots \
        CROSS JOIN LATERAL unnest($1) AS query_terms(query_term) \
        WHERE \
            ident % query_term \
            AND ident ILIKE '%' || query_term || '%' \
        GROUP BY id \
        ORDER BY min(ident <-> query_term) \
        LIMIT $2"
    )
    .bind(&query_terms)
    .bind(limit)
    .fetch_all(db_pool)
    .await
}

async fn search_by_full_text(
    db_pool: &PgPool,
    query: &str,
    limit: i32
) -> sqlx::Result<Vec<RobotPreview>>
{
    sqlx::query_as(
        "SELECT \
            id, robot_number, ident, prefix, suffix, plural, content_warning, image_thumb_path, \
            alt, custom_alt \
        FROM robots
        WHERE ts @@ replace(plainto_tsquery('english', $1)::text, '&', '|')::tsquery
        ORDER BY ts_rank(ts, replace(plainto_tsquery('english', $1)::text, '&', '|')::tsquery) DESC
        LIMIT $2"
    )
    .bind(query)
    .bind(limit)
    .fetch_all(db_pool)
    .await
}
