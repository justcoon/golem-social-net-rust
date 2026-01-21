use golem_rust::{agent_definition, agent_implementation, Schema};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Schema, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub comment_id: String,
    pub parent_comment_id: Option<String>,
    pub content: String,
    pub created_by: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Schema, Clone, Serialize, Deserialize)]
pub struct Post {
    pub post_id: String,
    pub content: String,
    pub created_by: String,
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
            created_at: now,
            updated_at: now,
        }
    }
}

#[agent_definition]
trait PostAgent {
    fn new(id: String) -> Self;

    fn get_post(&self) -> Option<Post>;

    fn init_post(&mut self, user_id: String, content: String) -> Result<(), String>;

    fn add_comment(
        &mut self,
        user_id: String,
        content: String,
        parent_comment_id: Option<String>,
    ) -> Result<String, String>;
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

    fn init_post(&mut self, user_id: String, content: String) -> Result<(), String> {
        if self.state.is_some() {
            Err("Post already exists".to_string())
        } else {
            self.with_state(|state| {
                let now = chrono::Utc::now();
                state.created_by = user_id;
                state.content = content;
                state.created_at = now;
                state.updated_at = now;
                Ok(())
            })
        }
    }

    fn add_comment(
        &mut self,
        user_id: String,
        content: String,
        parent_comment_id: Option<String>,
    ) -> Result<String, String> {
        self.with_state(|state| match parent_comment_id {
            Some(parent_comment_id) if !state.comments.contains_key(&parent_comment_id) => {
                Err("Parent comment not found".to_string())
            }
            _ => {
                let comment_id = uuid::Uuid::new_v4().to_string();
                let now = chrono::Utc::now();
                state.comments.insert(
                    comment_id.clone(),
                    Comment {
                        comment_id: comment_id.clone(),
                        parent_comment_id,
                        content,
                        created_by: user_id,
                        created_at: now,
                        updated_at: now,
                    },
                );
                Ok(comment_id)
            }
        })
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
