use rand::prelude::SliceRandom;

pub fn get_user_ids() -> Vec<String> {
    (1..=100).map(|v| format!("u{:03}", v)).collect()
}

pub fn rand_user_id() -> String {
    let user_ids = get_user_ids();
    user_ids
        .choose(&mut rand::thread_rng())
        .unwrap()
        .to_string()
}

pub fn rand_search_query() -> String {
    let queries = vec!["name:\"User\"", "email:\"test.com\"", "u00", "u01"];
    queries.choose(&mut rand::thread_rng()).unwrap().to_string()
}

pub fn rand_post_content() -> String {
    let contents = vec![
        "Hello social network!",
        "Check out my new post.",
        "Golem is amazing.",
        "Rust is the best language.",
    ];
    contents
        .choose(&mut rand::thread_rng())
        .unwrap()
        .to_string()
}

pub fn rand_comment_content() -> String {
    let contents = vec![
        "Nice post!",
        "I agree.",
        "Interesting point.",
        "Keep it up!",
    ];
    contents
        .choose(&mut rand::thread_rng())
        .unwrap()
        .to_string()
}

pub fn rand_message_content() -> String {
    let contents = vec![
        "Hey, how are you?",
        "Did you see the latest update?",
        "Let's meet tomorrow.",
        "That's funny!",
        "I'm on my way.",
    ];
    contents
        .choose(&mut rand::thread_rng())
        .unwrap()
        .to_string()
}

pub fn rand_user_ids(count: usize) -> Vec<String> {
    let user_ids = get_user_ids();
    user_ids
        .choose_multiple(&mut rand::thread_rng(), count)
        .cloned()
        .collect()
}

pub fn rand_like_type() -> crate::domain::common::LikeType {
    let types = vec![
        crate::domain::common::LikeType::Like,
        crate::domain::common::LikeType::Love,
        crate::domain::common::LikeType::Insightful,
        crate::domain::common::LikeType::Dislike,
    ];
    types.choose(&mut rand::thread_rng()).unwrap().to_owned()
}
