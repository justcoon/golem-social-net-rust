use crate::common::query;
use crate::post::{Post, PostAgentClient};
use futures::future::join_all;
use golem_rust::{agent_definition, agent_implementation, Schema};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

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
trait UserPostsViewAgent {
    fn new() -> Self;

    async fn get_posts_view(&mut self, user_id: String, query: String) -> Option<Vec<Post>>;
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
            let query_matcher = PostQueryMatcher::new(&query);

            println!("get posts view - user id: {user_id}, query matcher: {query_matcher}");

            let user_posts = user_posts.posts;

            if user_posts.is_empty() {
                Some(vec![])
            } else {
                let mut result: Vec<Post> = vec![];

                for chunk in user_posts.chunks(10) {
                    let clients = chunk
                        .iter()
                        .map(|p| PostAgentClient::get(p.post_id.clone()))
                        .collect::<Vec<_>>();

                    let tasks: Vec<_> = clients.iter().map(|client| client.get_post()).collect();

                    let responses = join_all(tasks).await;

                    let chunk_result: Vec<Post> = responses
                        .into_iter()
                        .flatten()
                        .filter(|p| query_matcher.matches_post(p.clone()))
                        .collect();

                    result.extend(chunk_result);
                }

                Some(result)
            }
        } else {
            None
        }
    }
}
