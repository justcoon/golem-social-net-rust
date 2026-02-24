use golem_rust::Schema;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::thread;
use std::time::Duration;
use std::time::Instant;

#[derive(Schema, Clone, Serialize, Deserialize, Debug, Hash, Eq, PartialEq)]
pub enum UserConnectionType {
    Friend,
    Follower,
    Following,
}

impl UserConnectionType {
    pub fn get_opposite(&self) -> UserConnectionType {
        match self {
            UserConnectionType::Follower => UserConnectionType::Following,
            UserConnectionType::Following => UserConnectionType::Follower,
            UserConnectionType::Friend => UserConnectionType::Friend,
        }
    }
}

impl Display for UserConnectionType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            UserConnectionType::Friend => write!(f, "Friend"),
            UserConnectionType::Follower => write!(f, "Follower"),
            UserConnectionType::Following => write!(f, "Following"),
        }
    }
}

#[derive(Schema, Clone, Serialize, Deserialize, Debug, Hash, Eq, PartialEq)]
pub enum LikeType {
    Like,
    Insightful,
    Love,
    Dislike,
}

impl LikeType {
    pub fn is_positive(&self) -> bool {
        !self.is_negative()
    }

    pub fn is_negative(&self) -> bool {
        matches!(self, LikeType::Dislike)
    }
}

impl Display for LikeType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            LikeType::Like => write!(f, "Like"),
            LikeType::Insightful => write!(f, "Insightful"),
            LikeType::Love => write!(f, "Love"),
            LikeType::Dislike => write!(f, "Dislike"),
        }
    }
}

pub(crate) mod query {
    use golem_rust::Schema;
    use std::fmt::{Display, Formatter};

    pub fn opt_text_matches(text: Option<String>, query: &str) -> bool {
        query == "*" || text.is_some_and(|text| text.to_lowercase().contains(&query.to_lowercase()))
    }

    pub fn opt_text_exact_matches(text: Option<String>, query: &str) -> bool {
        query == "*" || text.is_some_and(|text| text == query)
    }

    pub fn text_matches(text: &str, query: &str) -> bool {
        query == "*" || text.to_lowercase().contains(&query.to_lowercase())
    }

    pub fn text_exact_matches(text: &str, query: &str) -> bool {
        query == "*" || text == query
    }

    // Tokenize the query string, handling quoted strings
    pub fn tokenize(query: &str) -> Vec<String> {
        let mut tokens = Vec::new();
        let mut current = String::new();
        let mut in_quotes = false;

        for c in query.chars() {
            match c {
                ' ' if !in_quotes => {
                    if !current.is_empty() {
                        tokens.push(current.trim().to_string());
                        current.clear();
                    }
                }
                '"' => {
                    in_quotes = !in_quotes;
                }
                _ => {
                    current.push(c);
                }
            }
        }

        if !current.is_empty() {
            tokens.push(current.trim().to_string());
        }

        tokens
    }

    #[derive(Schema, Clone, Debug)]
    pub struct Query {
        pub terms: Vec<String>,
        pub field_filters: Vec<(String, String)>,
    }

    impl Display for Query {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                "Query(terms: {:?}, field_filters: {:?})",
                self.terms, self.field_filters
            )
        }
    }

    impl Query {
        pub fn new(query: &str) -> Self {
            let mut terms = Vec::new();
            let mut field_filters = Vec::new();

            let tokens = tokenize(query);

            for part in tokens {
                if let Some((field, value)) = part.split_once(':') {
                    field_filters.push((field.to_lowercase().to_string(), value.to_string()));
                } else {
                    terms.push(part.to_string());
                }
            }

            Self {
                terms,
                field_filters,
            }
        }
    }
}

pub(crate) mod snapshot {
    use serde::{de, Serialize};

    pub const SERIALIZATION_VERSION_V1: u8 = 1u8;

    pub(crate) fn serialize<T>(value: &T) -> Result<Vec<u8>, String>
    where
        T: ?Sized + Serialize,
    {
        let data = serde_json::to_vec_pretty(value).map_err(|err| err.to_string())?;

        let mut result = vec![SERIALIZATION_VERSION_V1];
        result.extend(data);

        Ok(result)
    }

    pub(crate) fn deserialize<'a, T>(bytes: &'a [u8]) -> Result<T, String>
    where
        T: de::Deserialize<'a>,
    {
        let (version, data) = bytes.split_at(1);

        match version[0] {
            SERIALIZATION_VERSION_V1 => {
                let value: T = serde_json::from_slice(data).map_err(|err| err.to_string())?;

                Ok(value)
            }
            _ => Err("Unsupported serialization version".to_string()),
        }
    }
}

pub async fn poll_for_updates<T, F, Fut>(
    user_id: String,
    updates_since: Option<chrono::DateTime<chrono::Utc>>,
    iter_wait_time: Option<u32>,
    max_wait_time: Option<u32>,
    get_updates_fn: F,
    log_prefix: &str,
) -> Option<Vec<T>>
where
    F: Fn(String, chrono::DateTime<chrono::Utc>) -> Fut,
    Fut: std::future::Future<Output = Option<Vec<T>>>,
{
    let since = updates_since.unwrap_or(chrono::Utc::now());
    let max_wait_time = Duration::from_millis(max_wait_time.unwrap_or(10000) as u64);
    let iter_wait_time = Duration::from_millis(iter_wait_time.unwrap_or(1000) as u64);
    let now = Instant::now();
    let mut done = false;
    let mut result: Option<Vec<T>> = None;

    while !done {
        println!(
            "{} - user id: {}, updates since: {}, elapsed time: {}ms, max wait time: {}ms",
            log_prefix,
            user_id,
            since,
            now.elapsed().as_millis(),
            max_wait_time.as_millis()
        );

        let res = get_updates_fn(user_id.clone(), since).await;

        if let Some(updates) = res {
            if !updates.is_empty() {
                result = Some(updates);
                done = true;
            } else {
                result = Some(vec![]);
                done = now.elapsed() >= max_wait_time;
                if !done {
                    thread::sleep(iter_wait_time);
                }
            }
        } else {
            result = None;
            done = true;
        }
    }
    result
}
