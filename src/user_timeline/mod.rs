use crate::common::query::Query;
use crate::common::UserConnectionType;
use crate::common::{poll_for_updates, query};
use crate::post::{fetch_posts_by_ids, fetch_posts_by_ids_and_query, Post};
use golem_rust::{agent_definition, agent_implementation, Schema};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

// max number of posts in timeline
const POSTS_MAX_COUNT: usize = 500;

#[derive(Schema, Clone, Serialize, Deserialize)]
pub struct PostRef {
    pub post_id: String,
    pub created_by: String,
    pub created_by_connection_type: Option<UserConnectionType>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl PostRef {
    pub fn new(
        post_id: String,
        created_by: String,
        created_at: chrono::DateTime<chrono::Utc>,
        created_by_connection_type: Option<UserConnectionType>,
        updated_at: chrono::DateTime<chrono::Utc>,
    ) -> Self {
        PostRef {
            post_id,
            created_by,
            created_by_connection_type,
            created_at,
            updated_at,
        }
    }

    fn matches_query(&self, query: Query) -> bool {
        // Check field filters first
        for (field, value) in query.field_filters.iter() {
            let matches = match field.as_str() {
                "post-id" | "postid" => query::text_exact_matches(&self.post_id, value),
                "connection-type" | "connectiontype" => query::opt_text_exact_matches(
                    self.created_by_connection_type
                        .clone()
                        .map(|t| t.to_string()),
                    value,
                ),
                "created-by" | "createdby" => query::text_exact_matches(&self.created_by, value),
                "content" => true,
                _ => false, // Unknown field
            };

            if !matches {
                return false;
            }
        }

        true
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
    fn add_or_update_posts(&mut self, posts: Vec<PostRef>) {
        let ids: HashSet<String> = posts.iter().map(|p| p.post_id.clone()).collect();

        self.posts.retain(|p| !ids.contains(&p.post_id));
        self.posts.extend(posts);

        self.posts
            .sort_by(|a, b| a.updated_at.cmp(&b.updated_at).reverse());

        // Keep only the first POSTS_MAX_COUNT elements
        if self.posts.len() > POSTS_MAX_COUNT {
            self.posts.truncate(POSTS_MAX_COUNT);
        }

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

#[derive(Schema, Clone, Serialize, Deserialize)]
pub struct UserTimelineUpdates {
    pub user_id: String,
    pub posts: Vec<PostRef>,
}

#[agent_definition]
trait UserTimelineAgent {
    fn new(id: String) -> Self;

    fn get_timeline(&self) -> Option<UserTimeline>;

    fn posts_updated(&mut self, posts: Vec<PostRef>) -> Result<(), String>;

    fn get_updates(
        &self,
        updates_since: chrono::DateTime<chrono::Utc>,
    ) -> Option<UserTimelineUpdates>;
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

    fn get_timeline(&self) -> Option<UserTimeline> {
        self.state.clone()
    }

    fn get_updates(
        &self,
        updates_since: chrono::DateTime<chrono::Utc>,
    ) -> Option<UserTimelineUpdates> {
        if let Some(state) = &self.state {
            println!("get updates - updates since: {updates_since}");

            let updates = state
                .posts
                .iter()
                .filter(|p| p.updated_at > updates_since)
                .cloned()
                .collect();

            Some(UserTimelineUpdates {
                user_id: state.user_id.clone(),
                posts: updates,
            })
        } else {
            None
        }
    }

    fn posts_updated(&mut self, posts: Vec<PostRef>) -> Result<(), String> {
        self.with_state(|state| {
            println!("posts updated - count: {}", posts.len());
            state.add_or_update_posts(posts);
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
trait UserTimelineViewAgent {
    fn new() -> Self;

    async fn get_posts_view(&mut self, user_id: String, query: String) -> Option<Vec<Post>>;

    async fn get_posts_updates_view(
        &mut self,
        user_id: String,
        updates_since: chrono::DateTime<chrono::Utc>,
    ) -> Option<Vec<Post>>;
}

struct UserTimelineViewAgentImpl {}

#[agent_implementation]
impl UserTimelineViewAgent for UserTimelineViewAgentImpl {
    fn new() -> Self {
        Self {}
    }

    async fn get_posts_view(&mut self, user_id: String, query: String) -> Option<Vec<Post>> {
        let timeline_posts = UserTimelineAgentClient::get(user_id.clone())
            .get_timeline()
            .await;

        println!("get posts view - user id: {user_id}, query: {query}");

        if let Some(timeline_posts) = timeline_posts {
            let query = query::Query::new(&query);

            println!("get posts view - user id: {user_id}, query matcher: {query}");

            let post_ids = timeline_posts
                .posts
                .into_iter()
                .filter(|p| p.matches_query(query.clone()))
                .map(|p| p.post_id)
                .collect::<Vec<_>>();

            if post_ids.is_empty() {
                Some(vec![])
            } else {
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
        let timeline_updates = UserTimelineAgentClient::get(user_id.clone())
            .get_updates(updates_since)
            .await;

        println!("get posts updates view - user id: {user_id}, updates since: {updates_since}");

        if let Some(timeline_updates) = timeline_updates {
            let updated_post_refs = timeline_updates.posts;

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

#[agent_definition(mode = "ephemeral")]
trait UserTimelineUpdatesAgent {
    fn new() -> Self;

    async fn get_posts_updates(
        &mut self,
        user_id: String,
        updates_since: Option<chrono::DateTime<chrono::Utc>>,
        iter_wait_time: Option<u32>,
        max_wait_time: Option<u32>,
    ) -> Option<Vec<PostRef>>;
}

struct UserTimelineUpdatesAgentImpl {}

#[agent_implementation]
impl UserTimelineUpdatesAgent for UserTimelineUpdatesAgentImpl {
    fn new() -> Self {
        Self {}
    }

    async fn get_posts_updates(
        &mut self,
        user_id: String,
        updates_since: Option<chrono::DateTime<chrono::Utc>>,
        iter_wait_time: Option<u32>,
        max_wait_time: Option<u32>,
    ) -> Option<Vec<PostRef>> {
        poll_for_updates(
            user_id,
            updates_since,
            iter_wait_time,
            max_wait_time,
            |uid, since| async move {
                let res = UserTimelineAgentClient::get(uid).get_updates(since).await;
                res.map(|r| r.posts)
            },
            "get posts updates",
        )
        .await
    }
}
