mod data;
mod domain;
mod goose_ext;

use crate::goose_ext::GooseRequestExt;
use goose::prelude::*;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), GooseError> {
    let custom_host = match std::env::var("HOST") {
        Ok(host) => host,
        Err(_) => "".to_string(),
    };

    GooseAttack::initialize()?
        .register_scenario(
            scenario!("Get User Data")
                .set_wait_time(Duration::from_secs(1), Duration::from_secs(5))?
                .register_transaction(transaction!(get_user_data)),
        )
        .register_scenario(
            scenario!("Search Users")
                .set_wait_time(Duration::from_secs(2), Duration::from_secs(10))?
                .register_transaction(transaction!(search_users)),
        )
        .register_scenario(
            scenario!("Get User Posts")
                .set_wait_time(Duration::from_secs(2), Duration::from_secs(10))?
                .register_transaction(transaction!(get_user_posts)),
        )
        .register_scenario(
            scenario!("Get User Timeline")
                .set_wait_time(Duration::from_secs(2), Duration::from_secs(10))?
                .register_transaction(transaction!(get_user_timeline)),
        )
        .register_scenario(
            scenario!("Get User Chat")
                .set_wait_time(Duration::from_secs(2), Duration::from_secs(10))?
                .register_transaction(transaction!(get_user_chat)),
        )
        .register_scenario(
            scenario!("Create Post, Comments and Likes")
                .set_wait_time(Duration::from_secs(5), Duration::from_secs(15))?
                .register_transaction(transaction!(create_post_comments_and_likes)),
        )
        .register_scenario(
            scenario!("Create Chat, Messages and Likes")
                .set_wait_time(Duration::from_secs(5), Duration::from_secs(15))?
                .register_transaction(transaction!(create_chat_messages_and_likes)),
        )
        .set_default(GooseDefault::Host, custom_host.as_str())?
        .execute()
        .await?;

    Ok(())
}

async fn get_user_data(user: &mut GooseUser) -> TransactionResult {
    let user_id = data::rand_user_id();

    let _response = user
        .get_request(
            "user-get",
            format!("/v1/social-net/users/{user_id}").as_str(),
        )
        .await?;

    Ok(())
}

async fn search_users(user: &mut GooseUser) -> TransactionResult {
    let query = data::rand_search_query();

    let _response = user
        .get_request(
            "user-search",
            format!("/v1/social-net/users/search?query={query}").as_str(),
        )
        .await?;

    Ok(())
}

async fn get_user_posts(user: &mut GooseUser) -> TransactionResult {
    let user_id = data::rand_user_id();

    let _response = user
        .get_request(
            "user-posts-get",
            format!("/v1/social-net/users/{user_id}/posts").as_str(),
        )
        .await?;

    Ok(())
}

async fn get_user_timeline(user: &mut GooseUser) -> TransactionResult {
    let user_id = data::rand_user_id();
    let query = data::rand_search_query();

    let _response = user
        .get_request(
            "user-timeline-get",
            format!("/v1/social-net/users/{user_id}/timeline/posts?query={query}").as_str(),
        )
        .await?;

    Ok(())
}

async fn get_user_chat(user: &mut GooseUser) -> TransactionResult {
    let user_id = data::rand_user_id();

    let _response = user
        .get_request(
            "user-chats-get",
            format!("/v1/social-net/users/{user_id}/chats").as_str(),
        )
        .await?;

    Ok(())
}

async fn create_post_comments_and_likes(user: &mut GooseUser) -> TransactionResult {
    use crate::goose_ext::GooseResponseExt;

    let user_id = data::rand_user_id();

    // 1. Create Post
    let create_post = domain::common::CreatePost {
        content: data::rand_post_content(),
    };
    let response = user
        .post_request(
            "post-create",
            format!("/v1/social-net/users/{user_id}/posts").as_str(),
            &create_post,
        )
        .await?;

    let post_created_res: domain::common::OkResult<domain::common::PostCreated> =
        response.json().await?;
    let post_id = post_created_res.ok.post_id;

    // 2. Like Post
    let set_post_like = domain::common::SetLike {
        user_id: data::rand_user_id(),
        like_type: data::rand_like_type(),
    };
    let _response = user
        .put_request(
            "post-like",
            format!("/v1/social-net/posts/{post_id}/likes").as_str(),
            &set_post_like,
        )
        .await?;

    // 3. Add Comments and Like them
    let mut last_comment_id = None;
    for _ in 0..2 {
        let comment_user_id = data::rand_user_id();
        let create_comment = domain::common::CreateComment {
            content: data::rand_comment_content(),
            user_id: comment_user_id.clone(),
            parent_comment_id: None,
        };

        let response = user
            .post_request(
                "comment-add",
                format!("/v1/social-net/posts/{post_id}/comments").as_str(),
                &create_comment,
            )
            .await?;

        let comment_id_res: domain::common::OkResult<String> = response.json().await?;
        let comment_id = comment_id_res.ok;
        last_comment_id = Some(comment_id.clone());

        // Like Comment
        let set_comment_like = domain::common::SetLike {
            user_id: data::rand_user_id(),
            like_type: data::rand_like_type(),
        };
        let _response = user
            .put_request(
                "comment-like",
                format!("/v1/social-net/posts/{post_id}/comments/{comment_id}/likes").as_str(),
                &set_comment_like,
            )
            .await?;
    }

    // 4. Delete one comment
    if let Some(comment_id) = last_comment_id {
        let _response = user
            .delete_request(
                "comment-delete",
                format!("/v1/social-net/posts/{post_id}/comments/{comment_id}").as_str(),
            )
            .await?;
    }

    Ok(())
}

async fn create_chat_messages_and_likes(user: &mut GooseUser) -> TransactionResult {
    use crate::goose_ext::GooseResponseExt;
    use rand::Rng;

    let creator_id = data::rand_user_id();
    let participant_count = rand::thread_rng().gen_range(1..5); // 1 to 4 additional participants (total 2 to 5)
    let participants = data::rand_user_ids(participant_count);

    // 1. Create Chat
    let create_chat = domain::common::CreateChat {
        participants: participants.clone(),
    };

    let response = user
        .post_request(
            "chat-create",
            format!("/v1/social-net/users/{creator_id}/chats").as_str(),
            &create_chat,
        )
        .await?;

    let chat_created_res: domain::common::OkResult<domain::common::ChatCreated> =
        response.json().await?;
    let chat_id = chat_created_res.ok.chat_id;

    // 2. Add Messages from each participant
    let mut all_participants = participants.clone();
    all_participants.push(creator_id.clone());

    let mut message_ids = Vec::new();

    for p_id in all_participants.iter() {
        for _ in 0..2 {
            let add_message = domain::common::AddMessage {
                user_id: p_id.clone(),
                content: data::rand_message_content(),
            };

            let response = user
                .post_request(
                    "chat-message-add",
                    format!("/v1/social-net/chats/{chat_id}/messages").as_str(),
                    &add_message,
                )
                .await?;

            let message_id_res: domain::common::OkResult<String> = response.json().await?;
            let message_id = message_id_res.ok;
            message_ids.push(message_id.clone());

            // 3. Like Message from 1-2 random users
            let like_count = rand::thread_rng().gen_range(1..3);
            let likers = data::rand_user_ids(like_count);

            for liker_id in likers {
                let set_like = domain::common::SetLike {
                    user_id: liker_id,
                    like_type: data::rand_like_type(),
                };

                let _response = user
                    .put_request(
                        "chat-message-like",
                        format!("/v1/social-net/chats/{chat_id}/messages/{message_id}/likes")
                            .as_str(),
                        &set_like,
                    )
                    .await?;
            }
        }
    }

    // 4. Delete some messages
    for message_id in message_ids.iter().take(2) {
        let _response = user
            .delete_request(
                "chat-message-delete",
                format!("/v1/social-net/chats/{chat_id}/messages/{message_id}").as_str(),
            )
            .await?;
    }

    Ok(())
}
