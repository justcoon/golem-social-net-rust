use crate::common::{LikeType, UserConnectionType};
use crate::user::UserAgentClient;
use crate::user_timeline::UserTimelineAgentClient;
use golem_rust::{agent_definition, agent_implementation, Schema};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const MAX_COMMENT_LENGTH: usize = 2000;

#[derive(Schema, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub comment_id: String,
    pub parent_comment_id: Option<String>,
    pub content: String,
    pub likes: HashMap<String, LikeType>,
    pub created_by: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl Comment {
    fn new(user_id: String, content: String, parent_comment_id: Option<String>) -> Self {
        let now = chrono::Utc::now();
        let comment_id = uuid::Uuid::new_v4().to_string();
        Comment {
            comment_id,
            parent_comment_id,
            content,
            likes: HashMap::new(),
            created_by: user_id,
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Schema, Clone, Serialize, Deserialize)]
pub struct Post {
    pub post_id: String,
    pub content: String,
    pub created_by: String,
    pub likes: HashMap<String, LikeType>,
    pub comments: HashMap<String, Comment>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl Post {
    fn new(post_id: String) -> Self {
        let now = chrono::Utc::now();
        Post {
            post_id,
            content: "".to_string(),
            comments: HashMap::new(),
            created_by: "".to_string(),
            likes: HashMap::new(),
            created_at: now,
            updated_at: now,
        }
    }

    fn add_comment(
        &mut self,
        user_id: String,
        content: String,
        parent_comment_id: Option<String>,
    ) -> Result<String, String> {
        match parent_comment_id {
            Some(parent_id) if !self.comments.contains_key(&parent_id) => {
                Err("Parent comment not found".to_string())
            }
            _ => {
                let comment = Comment::new(user_id.clone(), content, parent_comment_id);
                let comment_id = comment.comment_id.clone();

                self.comments.insert(comment_id.clone(), comment);

                self.updated_at = chrono::Utc::now();

                Ok(comment_id)
            }
        }
    }

    fn remove_comment(&mut self, comment_id: String) -> Result<(), String> {
        if !self.comments.contains_key(&comment_id) {
            Err("Comment not found".to_string())
        } else {
            fn collect_comments_to_remove(
                comments: &HashMap<String, Comment>,
                comment_id: &str,
            ) -> Vec<String> {
                let mut to_remove = Vec::new();

                // Add the current comment to the removal list
                to_remove.push(comment_id.to_string());

                // Find all child comments and recursively collect their descendants
                for comment in comments.values() {
                    if let Some(parent_id) = &comment.parent_comment_id {
                        if parent_id == comment_id {
                            to_remove
                                .extend(collect_comments_to_remove(comments, &comment.comment_id));
                        }
                    }
                }

                to_remove
            }

            // Recursively collect all comments to remove (children and their descendants)
            let to_remove = collect_comments_to_remove(&self.comments, &comment_id);

            // Remove all collected comments
            for remove_id in to_remove {
                self.comments.remove(&remove_id);
            }

            self.updated_at = chrono::Utc::now();

            Ok(())
        }
    }
}

#[agent_definition]
trait PostAgent {
    fn new(id: String) -> Self;

    fn get_post(&self) -> Option<Post>;

    async fn init_post(&mut self, user_id: String, content: String) -> Result<(), String>;

    fn add_comment(
        &mut self,
        user_id: String,
        content: String,
        parent_comment_id: Option<String>,
    ) -> Result<String, String>;

    fn remove_comment(&mut self, comment_id: String) -> Result<(), String>;

    fn set_like(&mut self, user_id: String, like_type: LikeType) -> Result<(), String>;

    fn remove_like(&mut self, user_id: String) -> Result<(), String>;

    fn set_comment_like(
        &mut self,
        comment_id: String,
        user_id: String,
        like_type: LikeType,
    ) -> Result<(), String>;

    fn remove_comment_like(&mut self, comment_id: String, user_id: String) -> Result<(), String>;
}

struct PostAgentImpl {
    _id: String,
    state: Option<Post>,
}

impl PostAgentImpl {
    fn get_state(&mut self) -> &mut Post {
        self.state.get_or_insert(Post::new(self._id.clone()))
    }

    fn with_state<T>(&mut self, f: impl FnOnce(&mut Post) -> T) -> T {
        f(self.get_state())
    }
}

#[agent_implementation]
impl PostAgent for PostAgentImpl {
    fn new(id: String) -> Self {
        PostAgentImpl {
            _id: id,
            state: None,
        }
    }

    fn get_post(&self) -> Option<Post> {
        self.state.clone()
    }

    async fn init_post(&mut self, user_id: String, content: String) -> Result<(), String> {
        if self.state.is_some() {
            Err("Post already exists".to_string())
        } else {
            let state = self.get_state();
            println!("init post - user id: {user_id}, content: {content}");
            let now = chrono::Utc::now();
            state.created_by = user_id.clone();
            state.content = content;
            state.created_at = now;
            state.updated_at = now;

            // let updated = TimelinesUpdaterAgentClient::get()
            //     .post_created(user_id.clone(), state.post_id.clone(), now)
            //     .await;
            //
            // println!("init post - user id: {user_id}, timelines updated: {updated}");

            // TimelinesUpdaterAgentClient::get()
            //     .trigger_post_created(user_id.clone(), state.post_id.clone(), now);

            TimelinesUpdaterAgentClient::new_phantom().trigger_post_created(
                user_id.clone(),
                state.post_id.clone(),
                now,
            );

            // execute_post_created_updates(user_id, state.post_id.clone(), now).await;
            Ok(())
        }
    }

    fn add_comment(
        &mut self,
        user_id: String,
        content: String,
        parent_comment_id: Option<String>,
    ) -> Result<String, String> {
        if self.state.is_none() {
            Err("Post not exists".to_string())
        } else {
            self.with_state(|state| {
                println!(
                    "add comment - user id: {}, content: {}, parent id: {}",
                    user_id,
                    content,
                    parent_comment_id.clone().unwrap_or("N/A".to_string())
                );
                if state.comments.len() >= MAX_COMMENT_LENGTH {
                    Err("Max comment length".to_string())
                } else {
                    state.add_comment(user_id.clone(), content, parent_comment_id)
                }
            })
        }
    }

    fn remove_comment(&mut self, comment_id: String) -> Result<(), String> {
        if self.state.is_none() {
            Err("Post not exists".to_string())
        } else {
            self.with_state(|state| {
                println!("remove comment - comment id: {}", comment_id);
                state.remove_comment(comment_id)
            })
        }
    }

    fn set_like(&mut self, user_id: String, like_type: LikeType) -> Result<(), String> {
        if self.state.is_none() {
            Err("Post not exists".to_string())
        } else {
            self.with_state(|state| {
                println!("set like - user id: {}, like type: {}", user_id, like_type);
                state.likes.insert(user_id, like_type);
                state.updated_at = chrono::Utc::now();
                Ok(())
            })
        }
    }

    fn remove_like(&mut self, user_id: String) -> Result<(), String> {
        if self.state.is_none() {
            Err("Post not exists".to_string())
        } else {
            self.with_state(|state| {
                println!("remove like - user id: {}", user_id);
                state.likes.remove(&user_id);
                state.updated_at = chrono::Utc::now();
                Ok(())
            })
        }
    }

    fn set_comment_like(
        &mut self,
        comment_id: String,
        user_id: String,
        like_type: LikeType,
    ) -> Result<(), String> {
        if self.state.is_none() {
            Err("Post not exists".to_string())
        } else {
            self.with_state(|state| {
                println!(
                    "set comment like - comment id: {}, user id: {}, like type: {}",
                    comment_id, user_id, like_type
                );

                match state.comments.get_mut(&comment_id) {
                    Some(comment) => {
                        comment.likes.insert(user_id, like_type);
                        comment.updated_at = chrono::Utc::now();
                        Ok(())
                    }
                    None => Err("Comment not found".to_string()),
                }
            })
        }
    }

    fn remove_comment_like(&mut self, comment_id: String, user_id: String) -> Result<(), String> {
        if self.state.is_none() {
            Err("Post not exists".to_string())
        } else {
            self.with_state(|state| {
                println!(
                    "remove comment like - comment id: {}, user id: {}",
                    comment_id, user_id
                );
                match state.comments.get_mut(&comment_id) {
                    Some(comment) => {
                        comment.likes.remove(&user_id);
                        comment.updated_at = chrono::Utc::now();
                        Ok(())
                    }
                    None => Err("Comment not found".to_string()),
                }
            })
        }
    }

    async fn load_snapshot(&mut self, bytes: Vec<u8>) -> Result<(), String> {
        let data: Option<Post> = crate::common::snapshot::deserialize(&bytes)?;
        self.state = data;
        Ok(())
    }

    async fn save_snapshot(&self) -> Result<Vec<u8>, String> {
        crate::common::snapshot::serialize(&self.state)
    }
}

// #[agent_definition(mode = "ephemeral")]
#[agent_definition]
trait TimelinesUpdaterAgent {
    fn new() -> Self;

    async fn post_created(
        &mut self,
        user_id: String,
        post_id: String,
        created_at: chrono::DateTime<chrono::Utc>,
    ) -> bool;
}

struct TimelinesUpdaterAgentImpl {}

#[agent_implementation]
impl TimelinesUpdaterAgent for TimelinesUpdaterAgentImpl {
    fn new() -> Self {
        Self {}
    }

    async fn post_created(
        &mut self,
        user_id: String,
        post_id: String,
        created_at: chrono::DateTime<chrono::Utc>,
    ) -> bool {
        execute_post_created_updates(user_id, post_id, created_at).await
    }
}

async fn execute_post_created_updates(
    user_id: String,
    post_id: String,
    created_at: chrono::DateTime<chrono::Utc>,
) -> bool {
    let user = UserAgentClient::get(user_id.clone()).get_user().await;

    if let Some(user) = user {
        println!("post created updates - user id: {user_id}, post id: {post_id}");
        UserTimelineAgentClient::get(user_id.clone()).trigger_add_post(
            post_id.clone(),
            user_id.clone(),
            created_at,
            None,
        );

        let mut notify_user_ids: HashMap<String, UserConnectionType> = HashMap::new();

        for (connected_user_id, connection) in user.connected_users {
            if connection
                .connection_types
                .contains(&UserConnectionType::Friend)
            {
                notify_user_ids.insert(connected_user_id, UserConnectionType::Friend);
            } else if connection
                .connection_types
                .contains(&UserConnectionType::Follower)
            {
                notify_user_ids.insert(connected_user_id, UserConnectionType::Follower);
            }
        }

        for (connected_user_id, connection_type) in notify_user_ids {
            UserTimelineAgentClient::get(connected_user_id).trigger_add_post(
                post_id.clone(),
                user_id.clone(),
                created_at,
                Some(connection_type),
            );
        }
        true
    } else {
        println!("post created updates - user id: {user_id} - not found");
        false
    }
}
