use crate::post::{Post, PostAgentClient};
use futures::future::join_all;
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
    fn contains_post(&self, post_id: String) -> bool {
        self.posts.iter().any(|p| p.post_id == post_id)
    }

    fn add_post(&mut self, post_id: String, created_by: String) {
        self.posts.push(PostRef::new(post_id, created_by));
        self.posts
            .sort_by(|a, b| a.created_at.cmp(&b.created_at).reverse());
        self.updated_at = chrono::Utc::now();
    }
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

    fn with_state<T>(&mut self, f: impl FnOnce(&mut UserTimeline) -> T) -> T {
        f(self.get_state())
    }
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
        self.with_state(|state| {
            println!("add post - id: {post_id}, created by: {created_by}");

            if !state.contains_post(post_id.clone()) {
                state.add_post(post_id, created_by);
            }

            Ok(())
        })
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

#[agent_definition(mode = "ephemeral")]
// #[agent_definition]
trait UserTimelineViewAgent {
    fn new() -> Self;

    async fn get_posts_view(&mut self, user_id: String) -> Option<Vec<Post>>;
}

struct UserTimelineViewAgentImpl {}

#[agent_implementation]
impl UserTimelineViewAgent for UserTimelineViewAgentImpl {
    fn new() -> Self {
        Self {}
    }

    async fn get_posts_view(&mut self, user_id: String) -> Option<Vec<Post>> {
        let timeline_posts = UserTimelineAgentClient::get(user_id).get_posts().await;

        if let Some(timeline_posts) = timeline_posts {
            let clients = timeline_posts
                .posts
                .iter()
                .map(|p| PostAgentClient::get(p.post_id.clone()))
                .collect::<Vec<_>>();

            let tasks: Vec<_> = clients.iter().map(|client| client.get_post()).collect();

            let responses = join_all(tasks).await;

            let result: Vec<Post> = responses.into_iter().flatten().collect();

            Some(result)
        } else {
            None
        }
    }
}
