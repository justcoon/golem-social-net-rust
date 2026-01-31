use crate::common::query;
use crate::common::UserConnectionType;
use crate::post::{Post, PostAgentClient};
use futures::future::join_all;
use golem_rust::{agent_definition, agent_implementation, Schema};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::{thread, time};

#[derive(Schema, Clone, Serialize, Deserialize)]
pub struct PostRef {
    pub post_id: String,
    pub created_by: String,
    pub created_by_connection_type: Option<UserConnectionType>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl PostRef {
    fn new(
        post_id: String,
        created_by: String,
        created_by_connection_type: Option<UserConnectionType>,
    ) -> Self {
        PostRef {
            post_id,
            created_by,
            created_by_connection_type,
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

    fn add_post(
        &mut self,
        post_id: String,
        created_by: String,
        created_by_connection_type: Option<UserConnectionType>,
    ) {
        self.posts.push(PostRef::new(
            post_id,
            created_by,
            created_by_connection_type,
        ));
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

#[derive(Schema, Clone, Serialize, Deserialize)]
pub struct UserTimelineUpdates {
    pub user_id: String,
    pub posts: Vec<PostRef>,
}

#[agent_definition]
trait UserTimelineAgent {
    fn new(id: String) -> Self;

    fn get_timeline(&self) -> Option<UserTimeline>;

    fn add_post(
        &mut self,
        post_id: String,
        created_by: String,
        by_connection_type: Option<UserConnectionType>,
    ) -> Result<(), String>;

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
                .filter(|p| p.created_at >= updates_since)
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

    fn add_post(
        &mut self,
        post_id: String,
        created_by: String,
        by_connection_type: Option<UserConnectionType>,
    ) -> Result<(), String> {
        self.with_state(|state| {
            println!("add post - id: {post_id}, created by: {created_by}");

            if !state.contains_post(post_id.clone()) {
                state.add_post(post_id, created_by, by_connection_type);
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

#[derive(Clone, Debug)]
struct PostQueryMatcher {
    terms: Vec<String>,
    field_filters: Vec<(String, String)>,
}

impl Display for PostQueryMatcher {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "PostQueryMatcher(terms: {:?}, field_filters: {:?})",
            self.terms, self.field_filters
        )
    }
}

impl PostQueryMatcher {
    fn new(query: &str) -> Self {
        let q = query::Query::new(query);

        Self {
            terms: q.terms,
            field_filters: q.field_filters,
        }
    }

    // Check if a post ref matches the query
    fn matches_post_ref(&self, post_ref: PostRef) -> bool {
        // Check field filters first
        for (field, value) in self.field_filters.iter() {
            let matches = match field.as_str() {
                "connection-type" | "connectiontype" => query::opt_text_exact_matches(
                    post_ref
                        .created_by_connection_type
                        .clone()
                        .map(|t| t.to_string()),
                    value,
                ),
                "created-by" | "createdby" => {
                    query::text_exact_matches(&post_ref.created_by, value)
                }
                "content" => true,
                _ => false, // Unknown field
            };

            if !matches {
                return false;
            }
        }

        true
    }

    // Check if a post matches the query
    fn matches_post(&self, post: Post) -> bool {
        // Check field filters first
        for (field, value) in self.field_filters.iter() {
            let matches = match field.as_str() {
                "created-by" | "createdby" => query::text_exact_matches(&post.created_by, value),
                "content" => query::text_matches(&post.content, value),
                "connection-type" | "connectiontype" => true,
                _ => false, // Unknown field
            };

            if !matches {
                return false;
            }
        }

        // If no terms to match, just check if field filters passed
        if self.terms.is_empty() {
            return true;
        }

        // Check search terms against all searchable fields
        for term in self.terms.iter() {
            let matches = query::text_matches(&post.content, term);

            if !matches {
                return false;
            }
        }

        true
    }
}

#[agent_definition(mode = "ephemeral")]
trait UserTimelineViewAgent {
    fn new() -> Self;

    async fn get_posts_view(&mut self, user_id: String, query: String) -> Option<Vec<Post>>;
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
            let query_matcher = PostQueryMatcher::new(&query);

            println!("get posts view - user id: {user_id}, query matcher: {query_matcher}");

            let timeline_posts = timeline_posts
                .posts
                .into_iter()
                .filter(|p| query_matcher.matches_post_ref(p.clone()))
                .collect::<Vec<_>>();

            if timeline_posts.is_empty() {
                Some(vec![])
            } else {
                let clients = timeline_posts
                    .iter()
                    .map(|p| PostAgentClient::get(p.post_id.clone()))
                    .collect::<Vec<_>>();

                let tasks: Vec<_> = clients.iter().map(|client| client.get_post()).collect();

                let responses = join_all(tasks).await;

                let result: Vec<Post> = responses
                    .into_iter()
                    .flatten()
                    .filter(|p| query_matcher.matches_post(p.clone()))
                    .collect();

                Some(result)
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
        max_wait_time: Option<u8>,
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
        max_wait_time: Option<u8>,
    ) -> Option<Vec<PostRef>> {
        let since = updates_since.unwrap_or(chrono::Utc::now());
        let max_wait_time = time::Duration::from_secs(max_wait_time.unwrap_or(10) as u64);
        let iter_wait_time = time::Duration::from_secs(1);
        let now = time::Instant::now();
        let mut done = false;
        let mut result: Option<Vec<PostRef>> = None;

        while !done {
            println!(
                "get posts updates - user id: {}, updates since: {}, elapsed time: {}ms, max wait time: {}ms",
                user_id,
                since,
                now.elapsed().as_millis(),
                max_wait_time.as_millis()
            );
            let res = UserTimelineAgentClient::get(user_id.clone())
                .get_updates(since)
                .await;

            if let Some(updates) = res {
                if !updates.posts.is_empty() {
                    result = Some(updates.posts);
                    done = true;
                } else {
                    result = Some(vec![]);
                    thread::sleep(iter_wait_time);
                    done = now.elapsed() >= max_wait_time;
                }
            } else {
                result = None;
                done = true;
            }
        }
        result
    }
}
