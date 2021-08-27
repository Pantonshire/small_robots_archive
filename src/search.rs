use std::collections::HashSet;

use lazy_static::lazy_static;
use regex::Regex;
use sqlx::postgres::PgPool;
use unidecode::unidecode;

use crate::respond::ResponseResult;
use crate::robots::RobotPreview;

lazy_static! {
    // Regex for matching "bot" at the end of a word
    static ref BOT_SUFFIX_RE: Regex = Regex::new(r"([Bb][^\w]*[Oo][^\w]*[Tt])([^\w]*[Ss][^\w]*)?$").unwrap();
}

const MAX_ROBOTS: i32 = 48;

//TODO: limit length of query string
//TODO: check for numbers in search query
//TODO: escape SQL wildcards

pub(crate) async fn search(db_pool: &PgPool, query: &str) -> ResponseResult<Vec<RobotPreview>> {
    let query_terms = {
        // Split the query by whitespace and convert to lowercase ascii
        let words = query
            .split_whitespace()
            .map(|word| {
                let mut word_lower_ascii = unidecode(word).to_lowercase();
                word_lower_ascii.retain(|c| !c.is_ascii_whitespace());
                word_lower_ascii
            })
            .collect::<Vec<_>>();

        let mut query_terms = Vec::new();

        for word in words {
            // Create a copy of any words ending with "bot", with the "bot" removed
            if let Some(re_match) = BOT_SUFFIX_RE.find(&word) {
                let trimmed_word = word[..re_match.start()].to_owned();
                query_terms.push(trimmed_word);
            }

            query_terms.push(word);
        }
        
        query_terms
    };

    // Vector for storing the robots found by the search
    let mut found_robots = Vec::new();

    // We only want to show each robot once, so keep track of the ids
    let mut found_ids = HashSet::new();

    let ident_matches: Vec<RobotPreview> = sqlx::query_as(
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
    .bind(MAX_ROBOTS)
    .fetch_all(db_pool)
    .await
    .map_err(actix_web::error::ErrorInternalServerError)?;

    for robot in ident_matches {
        found_ids.insert(robot.id);
        found_robots.push(robot);
    }

    let full_text_query = query_terms.join(" | ");

    let full_text_matches: Vec<RobotPreview> = sqlx::query_as(
        "SELECT \
            id, robot_number, ident, prefix, suffix, plural, content_warning, image_thumb_path, \
            alt, custom_alt \
        FROM robots
        WHERE ts @@ to_tsquery('english', $1)
        ORDER BY ts_rank(ts, to_tsquery('english', $1)) DESC
        LIMIT $2"
    )
    .bind(&full_text_query)
    .bind(MAX_ROBOTS - found_robots.len() as i32)
    .fetch_all(db_pool)
    .await
    .map_err(actix_web::error::ErrorInternalServerError)?;

    for robot in full_text_matches {
        if !found_ids.contains(&robot.id) {
            found_ids.insert(robot.id);
            found_robots.push(robot);
        }
    }

    Ok(found_robots)
}
