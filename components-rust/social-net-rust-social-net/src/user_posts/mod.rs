use crate::post::PostAgentClient;
use golem_rust::{agent_definition, agent_implementation, Schema};
use serde::{Deserialize, Serialize};

#[derive(Schema, Clone, Serialize, Deserialize)]
pub struct PostRef {
    pub post_id: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl PostRef {
    fn new(post_id: String) -> Self {
        PostRef {
            post_id,
            created_at: chrono::Utc::now(),
        }
    }
}

#[derive(Schema, Clone, Serialize, Deserialize)]
pub struct UserPosts {
    pub user_id: String,
    pub posts: Vec<PostRef>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl UserPosts {
    fn new(user_id: String) -> Self {
        let now = chrono::Utc::now();
        UserPosts {
            user_id,
            posts: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }
}

#[agent_definition]
trait UserPostsAgent {
    fn new(id: String) -> Self;

    fn get_posts(&self) -> Option<UserPosts>;

    fn create_post(&mut self, content: String) -> Result<String, String>;
}

struct UserPostTimelineAgentImpl {
    _id: String,
    state: Option<UserPosts>,
}

impl UserPostTimelineAgentImpl {
    fn get_state(&mut self) -> &mut UserPosts {
        self.state.get_or_insert(UserPosts::new(self._id.clone()))
    }

    fn with_state<T>(&mut self, f: impl FnOnce(&mut UserPosts) -> T) -> T {
        f(self.get_state())
    }
}

#[agent_implementation]
impl UserPostsAgent for UserPostTimelineAgentImpl {
    fn new(id: String) -> Self {
        UserPostTimelineAgentImpl {
            _id: id,
            state: None,
        }
    }

    fn get_posts(&self) -> Option<UserPosts> {
        self.state.clone()
    }

    fn create_post(&mut self, content: String) -> Result<String, String> {
        self.with_state(|state| {
            let post_id = uuid::Uuid::new_v4().to_string();

            println!("create post - id: {post_id}");

            let post_ref = PostRef::new(post_id.clone());

            PostAgentClient::get(post_id.clone()).trigger_init_post(state.user_id.clone(), content);

            state.posts.push(post_ref);

            Ok(post_id)
        })
    }

    async fn load_snapshot(&mut self, bytes: Vec<u8>) -> Result<(), String> {
        let data: Option<UserPosts> = crate::common::snapshot::deserialize(&bytes)?;
        self.state = data;
        Ok(())
    }

    async fn save_snapshot(&self) -> Result<Vec<u8>, String> {
        crate::common::snapshot::serialize(&self.state)
    }
}
