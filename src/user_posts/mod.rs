use crate::common::query;
use crate::post::{fetch_posts_by_ids, fetch_posts_by_ids_and_query, Post, PostAgentClient};
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

#[derive(Schema, Clone, Serialize, Deserialize)]
pub struct UserPostsUpdates {
    pub user_id: String,
    pub posts: Vec<PostRef>,
}

#[agent_definition]
trait UserPostsAgent {
    fn new(id: String) -> Self;

    fn get_posts(&self) -> Option<UserPosts>;

    fn get_updates(&self, updates_since: chrono::DateTime<chrono::Utc>)
        -> Option<UserPostsUpdates>;

    fn create_post(&mut self, content: String) -> Result<String, String>;
}

struct UserPostsAgentImpl {
    _id: String,
    state: Option<UserPosts>,
}

impl UserPostsAgentImpl {
    fn get_state(&mut self) -> &mut UserPosts {
        self.state.get_or_insert(UserPosts::new(self._id.clone()))
    }

    fn with_state<T>(&mut self, f: impl FnOnce(&mut UserPosts) -> T) -> T {
        f(self.get_state())
    }
}

#[agent_implementation]
impl UserPostsAgent for UserPostsAgentImpl {
    fn new(id: String) -> Self {
        UserPostsAgentImpl {
            _id: id,
            state: None,
        }
    }

    fn get_posts(&self) -> Option<UserPosts> {
        self.state.clone()
    }

    fn get_updates(
        &self,
        updates_since: chrono::DateTime<chrono::Utc>,
    ) -> Option<UserPostsUpdates> {
        if let Some(state) = &self.state {
            println!("get updates - updates since: {updates_since}");

            let updates = state
                .posts
                .iter()
                .filter(|p| p.created_at > updates_since)
                .cloned()
                .collect();

            Some(UserPostsUpdates {
                user_id: state.user_id.clone(),
                posts: updates,
            })
        } else {
            None
        }
    }

    fn create_post(&mut self, content: String) -> Result<String, String> {
        self.with_state(|state| {
            let post_id = uuid::Uuid::new_v4().to_string();

            println!("create post - id: {post_id}");

            let post_ref = PostRef::new(post_id.clone());

            PostAgentClient::get(post_id.clone()).trigger_init_post(state.user_id.clone(), content);

            state.updated_at = post_ref.created_at;
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

#[agent_definition(mode = "ephemeral")]
trait UserPostsViewAgent {
    fn new() -> Self;

    async fn get_posts_view(&mut self, user_id: String, query: String) -> Option<Vec<Post>>;

    async fn get_posts_updates_view(
        &mut self,
        user_id: String,
        updates_since: chrono::DateTime<chrono::Utc>,
    ) -> Option<Vec<Post>>;
}

struct UserPostsViewAgentImpl {}

#[agent_implementation]
impl UserPostsViewAgent for UserPostsViewAgentImpl {
    fn new() -> Self {
        Self {}
    }

    async fn get_posts_view(&mut self, user_id: String, query: String) -> Option<Vec<Post>> {
        let user_posts = UserPostsAgentClient::get(user_id.clone()).get_posts().await;

        println!("get posts view - user id: {user_id}, query: {query}");

        if let Some(user_posts) = user_posts {
            let query = query::Query::new(&query);

            println!("get posts view - user id: {user_id}, query matcher: {query}");

            let user_posts = user_posts.posts;

            if user_posts.is_empty() {
                Some(vec![])
            } else {
                let post_ids: Vec<String> = user_posts.iter().map(|p| p.post_id.clone()).collect();
                let posts = fetch_posts_by_ids_and_query(&post_ids, query).await;

                Some(posts)
            }
        } else {
            None
        }
    }

    async fn get_posts_updates_view(
        &mut self,
        user_id: String,
        updates_since: chrono::DateTime<chrono::Utc>,
    ) -> Option<Vec<Post>> {
        let user_posts_updates = UserPostsAgentClient::get(user_id.clone())
            .get_updates(updates_since)
            .await;

        println!("get posts updates view - user id: {user_id}, updates since: {updates_since}");

        if let Some(user_posts_updates) = user_posts_updates {
            let updated_post_refs = user_posts_updates.posts;

            if updated_post_refs.is_empty() {
                Some(vec![])
            } else {
                let post_ids: Vec<String> = updated_post_refs
                    .iter()
                    .map(|p| p.post_id.clone())
                    .collect();
                let posts = fetch_posts_by_ids(&post_ids).await;

                Some(posts)
            }
        } else {
            None
        }
    }
}
