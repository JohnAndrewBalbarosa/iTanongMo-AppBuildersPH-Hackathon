use std::collections::HashMap;

use sqlx::SqlitePool;

use crate::error::AppResult;

use super::Agency;

pub async fn search_agencies(pool: &SqlitePool, query: &str) -> AppResult<Vec<Agency>> {
    let agencies = super::list_agencies(pool).await?;
    let query_tokens = tokens(query);
    if query_tokens.is_empty() {
        return Ok(agencies);
    }
    let last_token_is_partial = !query.chars().last().is_some_and(char::is_whitespace);
    Ok(agencies
        .into_iter()
        .filter(|agency| agency_matches(agency, &query_tokens, last_token_is_partial))
        .collect())
}

fn agency_matches(agency: &Agency, query: &[String], last_token_is_partial: bool) -> bool {
    let sources = [
        tokens(&agency.name),
        tokens(&agency.short_name),
        tokens(&agency.slug.replace('-', " ")),
    ];
    sources
        .iter()
        .any(|candidate| multiset_contains(candidate, query, last_token_is_partial))
}

fn multiset_contains(candidate: &[String], query: &[String], last_is_partial: bool) -> bool {
    let mut available = frequencies(candidate);
    for (index, token) in query.iter().enumerate() {
        let partial = last_is_partial && index + 1 == query.len();
        let matched = available
            .iter()
            .filter(|(_, count)| **count > 0)
            .map(|(value, _)| value.clone())
            .find(|value| {
                if partial {
                    value.starts_with(token)
                } else {
                    value == token
                }
            });
        let Some(value) = matched else { return false };
        if let Some(count) = available.get_mut(&value) {
            *count -= 1;
        }
    }
    true
}

fn frequencies(values: &[String]) -> HashMap<String, usize> {
    let mut counts = HashMap::new();
    for value in values {
        *counts.entry(value.clone()).or_insert(0) += 1;
    }
    counts
}

fn tokens(value: &str) -> Vec<String> {
    value
        .to_lowercase()
        .split(|character: char| !character.is_alphanumeric())
        .filter(|token| !token.is_empty())
        .map(str::to_owned)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{multiset_contains, tokens};

    #[test]
    fn matching_is_case_and_order_independent() {
        let candidate = tokens("Department of Agriculture");
        assert!(multiset_contains(
            &candidate,
            &tokens("of department AgRiCultUre"),
            true
        ));
    }

    #[test]
    fn one_candidate_word_cannot_satisfy_duplicate_query_words() {
        let engineering = tokens("Bachelor of Science in Computer Engineering");
        let computer_science = tokens("Bachelor of Science in Computer Science");
        let query = tokens("Bachelor Science Computer Science");
        assert!(!multiset_contains(&engineering, &query, true));
        assert!(multiset_contains(&computer_science, &query, true));
        assert!(!multiset_contains(
            &computer_science,
            &tokens("Science Science Science"),
            true
        ));
    }
}
