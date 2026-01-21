use golem_rust::{agent_definition, agent_implementation, Schema};
use serde::{Deserialize, Serialize};

#[derive(Schema, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub comment_id: String,
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
    pub comments: Vec<Comment>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl Post {
    pub fn new(post_id: String) -> Self {
        Post {
            post_id,
            content: "".to_string(),
            comments: Vec::new(),
            created_by: "".to_string(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }
}

#[agent_definition]
trait PostAgent {
    fn new(id: String) -> Self;

    fn get_post(&self) -> Option<Post>;

    fn init_post(&mut self, user_id: String, content: String);

    fn add_comment(&mut self, user_id: String, content: String);
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

    fn init_post(&mut self, user_id: String, content: String) {
        self.with_state(|state| {
            state.created_by = user_id;
            state.content = content;
            state.created_at = chrono::Utc::now();
            state.updated_at = chrono::Utc::now();
        })
    }

    fn add_comment(&mut self, user_id: String, content: String) {
        self.with_state(|state| {
            state.comments.push(Comment {
                comment_id: uuid::Uuid::new_v4().to_string(),
                content,
                created_by: user_id,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            })
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
