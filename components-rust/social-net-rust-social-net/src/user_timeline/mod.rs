use golem_rust::{agent_definition, agent_implementation, Schema};
use serde::{Deserialize, Serialize};

#[derive(Schema, Clone, Serialize, Deserialize)]
pub struct PostRef {
    pub post_id: String,
    pub created_by: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl PostRef {
    fn new(post_id: String, created_by: String) -> Self {
        PostRef {
            post_id,
            created_by,
            created_at: chrono::Utc::now(),
        }
    }
}

#[derive(Schema, Clone, Serialize, Deserialize)]
pub struct UserTimeline {
    pub user_id: String,
    pub posts: Vec<PostRef>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl UserTimeline {
    fn new(user_id: String) -> Self {
        let now = chrono::Utc::now();
        UserTimeline {
            user_id,
            posts: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }
}

#[agent_definition]
trait UserTimelineAgent {
    fn new(id: String) -> Self;

    fn get_posts(&self) -> Option<UserTimeline>;

    fn add_post(&mut self, post_id: String, created_by: String) -> Result<(), String>;
}

struct UserTimelineAgentImpl {
    _id: String,
    state: Option<UserTimeline>,
}

impl UserTimelineAgentImpl {
    fn get_state(&mut self) -> &mut UserTimeline {
        self.state
            .get_or_insert(UserTimeline::new(self._id.clone()))
    }

    // fn with_state<T>(&mut self, f: impl FnOnce(&mut UserTimeline) -> T) -> T {
    //     f(self.get_state())
    // }
}

#[agent_implementation]
impl UserTimelineAgent for UserTimelineAgentImpl {
    fn new(id: String) -> Self {
        UserTimelineAgentImpl {
            _id: id,
            state: None,
        }
    }

    fn get_posts(&self) -> Option<UserTimeline> {
        self.state.clone()
    }

    fn add_post(&mut self, post_id: String, created_by: String) -> Result<(), String> {
        let state = self.get_state();

        println!("add post - id: {post_id}, created by: {created_by}");

        if !state.posts.iter().any(|p| p.post_id == post_id) {
            let post_ref = PostRef::new(post_id.clone(), created_by.clone());

            state.posts.push(post_ref);
        }

        Ok(())
    }

    async fn load_snapshot(&mut self, bytes: Vec<u8>) -> Result<(), String> {
        let data: Option<UserTimeline> = crate::common::snapshot::deserialize(&bytes)?;
        self.state = data;
        Ok(())
    }

    async fn save_snapshot(&self) -> Result<Vec<u8>, String> {
        crate::common::snapshot::serialize(&self.state)
    }
}
