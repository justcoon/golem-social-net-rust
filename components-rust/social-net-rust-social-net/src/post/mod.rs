use crate::common::query::Query;
use crate::common::{query, LikeType, UserConnectionType};
use crate::user::UserAgentClient;
use crate::user_timeline::{PostRef, UserTimelineAgentClient};
use futures::future::join_all;
use golem_rust::{agent_definition, agent_implementation, Schema};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// max number of comments
const COMMENTS_MAX_COUNT: usize = 2000;

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

    fn set_like(&mut self, user_id: String, like_type: LikeType) -> bool {
        let res = self.likes.insert(user_id, like_type);
        self.updated_at = chrono::Utc::now();
        res.is_some()
    }

    fn remove_like(&mut self, user_id: String) -> bool {
        let res = self.likes.remove(&user_id);
        if res.is_some() {
            self.updated_at = chrono::Utc::now();
        }
        res.is_some()
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

    fn set_comment_like(
        &mut self,
        comment_id: String,
        user_id: String,
        like_type: LikeType,
    ) -> Result<(), String> {
        match self.comments.get_mut(&comment_id) {
            Some(comment) => {
                comment.likes.insert(user_id, like_type);
                comment.updated_at = chrono::Utc::now();
                Ok(())
            }
            None => Err("Comment not found".to_string()),
        }
    }

    fn remove_comment_like(&mut self, comment_id: String, user_id: String) -> Result<(), String> {
        match self.comments.get_mut(&comment_id) {
            Some(comment) => {
                let removed = comment.likes.remove(&user_id).is_some();
                if removed {
                    comment.updated_at = chrono::Utc::now();
                }
                Ok(())
            }
            None => Err("Comment not found".to_string()),
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

            TimelinesUpdaterAgentClient::get(user_id.clone())
                .trigger_post_updated(PostUpdate::from(state), true);

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
                if state.comments.len() >= COMMENTS_MAX_COUNT {
                    Err("Max comment length".to_string())
                } else {
                    let comment_id =
                        state.add_comment(user_id.clone(), content, parent_comment_id)?;
                    TimelinesUpdaterAgentClient::get(user_id.clone())
                        .trigger_post_updated(PostUpdate::from(state), false);
                    Ok(comment_id)
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
                state.remove_comment(comment_id)?;
                TimelinesUpdaterAgentClient::get(state.created_by.clone())
                    .trigger_post_updated(PostUpdate::from(state), false);
                Ok(())
            })
        }
    }

    fn set_like(&mut self, user_id: String, like_type: LikeType) -> Result<(), String> {
        if self.state.is_none() {
            Err("Post not exists".to_string())
        } else {
            self.with_state(|state| {
                println!("set like - user id: {}, like type: {}", user_id, like_type);
                state.set_like(user_id, like_type);
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
                state.remove_like(user_id);
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

                state.set_comment_like(comment_id, user_id, like_type)
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
                state.remove_comment_like(comment_id, user_id)
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

#[derive(Schema, Clone, Serialize, Deserialize)]
pub struct PostUpdate {
    pub post_id: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl PostUpdate {
    fn from(value: &Post) -> Self {
        PostUpdate {
            post_id: value.post_id.clone(),
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

#[derive(Schema, Clone, Serialize, Deserialize)]
pub struct PostUpdates {
    pub user_id: String,
    pub updates: Vec<PostUpdate>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl PostUpdates {
    fn new(user_id: String) -> Self {
        let now = chrono::Utc::now();
        Self {
            user_id,
            updates: vec![],
            created_at: now,
            updated_at: now,
        }
    }
}

#[agent_definition]
trait TimelinesUpdaterAgent {
    fn new(id: String) -> Self;

    fn get_updates(&self) -> PostUpdates;

    async fn post_updated(&mut self, update: PostUpdate, process_immediately: bool);

    async fn process_posts_updates(&mut self);
}

struct TimelinesUpdaterAgentImpl {
    state: PostUpdates,
}
impl TimelinesUpdaterAgentImpl {
    async fn execute_posts_updates(&mut self) {
        if !self.state.updates.is_empty() {
            execute_posts_updates(self.state.user_id.clone(), self.state.updates.clone()).await;
            self.state.updates.clear();
            self.state.updated_at = chrono::Utc::now();
        }
    }

    fn add_update(&mut self, update: PostUpdate) {
        self.state.updates.retain(|x| x.post_id != update.post_id);
        self.state.updates.push(update);
        self.state.updated_at = chrono::Utc::now();
    }
}

#[agent_implementation]
impl TimelinesUpdaterAgent for TimelinesUpdaterAgentImpl {
    fn new(id: String) -> Self {
        Self {
            state: PostUpdates::new(id),
        }
    }

    fn get_updates(&self) -> PostUpdates {
        self.state.clone()
    }

    async fn post_updated(&mut self, update: PostUpdate, process_immediately: bool) {
        println!(
            "post updates - user id: {}, post id: {}",
            self.state.user_id.clone(),
            update.post_id.clone()
        );
        self.add_update(update);

        if process_immediately {
            println!(
                "post updates - user id: {}, updates: {} - processing ...",
                self.state.user_id.clone(),
                self.state.updates.len()
            );
            self.execute_posts_updates().await;
        }
    }

    async fn process_posts_updates(&mut self) {
        println!(
            "posts updates - user id: {}, updates: {} - processing ...",
            self.state.user_id.clone(),
            self.state.updates.len()
        );
        self.execute_posts_updates().await;
    }

    async fn load_snapshot(&mut self, bytes: Vec<u8>) -> Result<(), String> {
        let data: PostUpdates = crate::common::snapshot::deserialize(&bytes)?;
        self.state = data;
        Ok(())
    }

    async fn save_snapshot(&self) -> Result<Vec<u8>, String> {
        crate::common::snapshot::serialize(&self.state)
    }
}

async fn execute_posts_updates(user_id: String, updates: Vec<PostUpdate>) -> bool {
    let user = UserAgentClient::get(user_id.clone()).get_user().await;

    if let Some(user) = user {
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

        println!(
            "posts updates - user id: {user_id} - updates: {}, notify users: {}",
            updates.len(),
            notify_user_ids.len()
        );
        execute_posts_update(user_id.clone(), updates, notify_user_ids.clone());

        true
    } else {
        println!("posts updates - user id: {user_id} - not found");
        false
    }
}

fn execute_posts_update(
    user_id: String,
    updates: Vec<PostUpdate>,
    notify_user_ids: HashMap<String, UserConnectionType>,
) {
    let user_updates = updates
        .clone()
        .into_iter()
        .map(|update| {
            PostRef::new(
                update.post_id.clone(),
                user_id.clone(),
                update.created_at,
                None,
                update.updated_at,
            )
        })
        .collect();

    UserTimelineAgentClient::get(user_id.clone()).trigger_posts_updated(user_updates);

    for (connected_user_id, connection_type) in notify_user_ids {
        let user_updates = updates
            .clone()
            .into_iter()
            .map(|update| {
                PostRef::new(
                    update.post_id.clone(),
                    user_id.clone(),
                    update.created_at,
                    Some(connection_type.clone()),
                    update.updated_at,
                )
            })
            .collect();
        UserTimelineAgentClient::get(connected_user_id).trigger_posts_updated(user_updates);
    }
}

pub async fn fetch_posts_by_ids(post_ids: &[String]) -> Vec<Post> {
    let mut result: Vec<Post> = vec![];

    for chunk in post_ids.chunks(10) {
        let clients = chunk
            .iter()
            .map(|post_id| PostAgentClient::get(post_id.clone()))
            .collect::<Vec<_>>();

        let tasks: Vec<_> = clients.iter().map(|client| client.get_post()).collect();
        let responses = join_all(tasks).await;

        let chunk_result: Vec<Post> = responses.into_iter().flatten().collect();

        result.extend(chunk_result);
    }

    result
}

// Check if a post matches the query
pub fn matches_post(post: Post, query: Query) -> bool {
    // Check field filters first
    for (field, value) in query.field_filters.iter() {
        let matches = match field.as_str() {
            "created-by" | "createdby" => query::text_exact_matches(&post.created_by, value),
            "content" => query::text_matches(&post.content, value),
            "connection-type" | "connectiontype" => true,
            "comments" => post
                .comments
                .iter()
                .any(|(_, c)| query::text_matches(&c.content, value)),
            _ => false, // Unknown field
        };

        if !matches {
            return false;
        }
    }

    // If no terms to match, just check if field filters passed
    if query.terms.is_empty() {
        return true;
    }

    // Check search terms against all searchable fields
    for term in query.terms.iter() {
        let matches = query::text_matches(&post.content, term);

        if !matches {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::LikeType;

    fn create_test_post() -> Post {
        let mut post = Post::new("test-post-1".to_string());
        post.created_by = "user1".to_string();
        post.content = "Test post content".to_string();
        post
    }

    #[test]
    fn test_post_new() {
        let post = Post::new("test-post".to_string());
        assert_eq!(post.post_id, "test-post");
        assert_eq!(post.content, "");
        assert_eq!(post.created_by, "");
        assert!(post.likes.is_empty());
        assert!(post.comments.is_empty());
        assert_eq!(post.created_at, post.updated_at);
    }

    #[test]
    fn test_set_like_new_user() {
        let mut post = create_test_post();
        let initial_updated_at = post.updated_at;

        let result = post.set_like("user2".to_string(), LikeType::Like);

        assert!(!result); // First time like, returns false (no previous like)
        assert_eq!(post.likes.len(), 1);
        assert_eq!(post.likes.get("user2"), Some(&LikeType::Like));
        assert!(post.updated_at > initial_updated_at);
    }

    #[test]
    fn test_set_like_override_existing() {
        let mut post = create_test_post();

        // Add initial like
        post.set_like("user2".to_string(), LikeType::Like);
        let initial_updated_at = post.updated_at;

        // Override with different like type
        let result = post.set_like("user2".to_string(), LikeType::Love);

        assert!(result); // Override, returns true (previous like existed)
        assert_eq!(post.likes.len(), 1);
        assert_eq!(post.likes.get("user2"), Some(&LikeType::Love));
        assert!(post.updated_at > initial_updated_at);
    }

    #[test]
    fn test_remove_like_success() {
        let mut post = create_test_post();

        // Add a like first
        post.set_like("user2".to_string(), LikeType::Like);
        assert_eq!(post.likes.len(), 1);

        let initial_updated_at = post.updated_at;

        // Remove the like
        let result = post.remove_like("user2".to_string());

        assert!(result);
        assert_eq!(post.likes.len(), 0);
        assert!(post.updated_at > initial_updated_at);
    }

    #[test]
    fn test_remove_like_not_found() {
        let mut post = create_test_post();
        let initial_updated_at = post.updated_at;

        // Try to remove non-existent like
        let result = post.remove_like("user2".to_string());

        assert!(!result);
        assert_eq!(post.likes.len(), 0);
        assert_eq!(post.updated_at, initial_updated_at);
    }

    #[test]
    fn test_add_comment_success() {
        let mut post = create_test_post();
        let initial_updated_at = post.updated_at;

        // Add root comment
        let result = post.add_comment("user2".to_string(), "Great post!".to_string(), None);

        assert!(result.is_ok());
        let comment_id = result.unwrap();
        assert_eq!(post.comments.len(), 1);

        let comment = post.comments.get(&comment_id).unwrap();
        assert_eq!(comment.content, "Great post!");
        assert_eq!(comment.created_by, "user2");
        assert!(comment.parent_comment_id.is_none());
        assert!(comment.likes.is_empty());
        assert!(post.updated_at > initial_updated_at);
    }

    #[test]
    fn test_add_comment_with_parent() {
        let mut post = create_test_post();

        // Add parent comment first
        let parent_id = post
            .add_comment("user2".to_string(), "Parent comment".to_string(), None)
            .unwrap();

        // Add child comment
        let result = post.add_comment(
            "user3".to_string(),
            "Child comment".to_string(),
            Some(parent_id.clone()),
        );

        assert!(result.is_ok());
        let child_id = result.unwrap();
        assert_eq!(post.comments.len(), 2);

        let child_comment = post.comments.get(&child_id).unwrap();
        assert_eq!(child_comment.content, "Child comment");
        assert_eq!(child_comment.parent_comment_id, Some(parent_id));
    }

    #[test]
    fn test_add_comment_parent_not_found() {
        let mut post = create_test_post();

        // Try to add comment with non-existent parent
        let result = post.add_comment(
            "user2".to_string(),
            "Orphan comment".to_string(),
            Some("non-existent".to_string()),
        );

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Parent comment not found");
        assert_eq!(post.comments.len(), 0);
    }

    #[test]
    fn test_remove_comment_success() {
        let mut post = create_test_post();

        // Add a comment first
        let comment_id = post
            .add_comment("user2".to_string(), "Test comment".to_string(), None)
            .unwrap();
        assert_eq!(post.comments.len(), 1);

        let initial_updated_at = post.updated_at;

        // Remove the comment
        let result = post.remove_comment(comment_id.clone());

        assert!(result.is_ok());
        assert_eq!(post.comments.len(), 0);
        assert!(post.updated_at > initial_updated_at);
    }

    #[test]
    fn test_remove_comment_not_found() {
        let mut post = create_test_post();
        let initial_updated_at = post.updated_at;

        // Try to remove non-existent comment
        let result = post.remove_comment("non-existent".to_string());

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Comment not found");
        assert_eq!(post.comments.len(), 0);
        assert_eq!(post.updated_at, initial_updated_at);
    }

    #[test]
    fn test_remove_comment_with_children() {
        let mut post = create_test_post();

        // Add parent comment
        let parent_id = post
            .add_comment("user2".to_string(), "Parent comment".to_string(), None)
            .unwrap();

        // Add child comment
        let child_id = post
            .add_comment(
                "user3".to_string(),
                "Child comment".to_string(),
                Some(parent_id.clone()),
            )
            .unwrap();

        // Add grandchild comment
        let grandchild_id = post
            .add_comment(
                "user4".to_string(),
                "Grandchild comment".to_string(),
                Some(child_id.clone()),
            )
            .unwrap();

        assert_eq!(post.comments.len(), 3);

        // Remove parent comment (should remove all descendants)
        let result = post.remove_comment(parent_id.clone());

        assert!(result.is_ok());
        assert_eq!(post.comments.len(), 0);

        // Verify all comments are removed
        assert!(!post.comments.contains_key(&parent_id));
        assert!(!post.comments.contains_key(&child_id));
        assert!(!post.comments.contains_key(&grandchild_id));
    }

    #[test]
    fn test_remove_child_comment_only() {
        let mut post = create_test_post();

        // Add parent comment
        let parent_id = post
            .add_comment("user2".to_string(), "Parent comment".to_string(), None)
            .unwrap();

        // Add child comment
        let child_id = post
            .add_comment(
                "user3".to_string(),
                "Child comment".to_string(),
                Some(parent_id.clone()),
            )
            .unwrap();

        assert_eq!(post.comments.len(), 2);

        // Remove only child comment
        let result = post.remove_comment(child_id.clone());

        assert!(result.is_ok());
        assert_eq!(post.comments.len(), 1);

        // Verify parent remains, child is removed
        assert!(post.comments.contains_key(&parent_id));
        assert!(!post.comments.contains_key(&child_id));
    }

    #[test]
    fn test_set_comment_like_success() {
        let mut post = create_test_post();
        let comment_id = post
            .add_comment("user2".to_string(), "Test comment".to_string(), None)
            .unwrap();
        let initial_updated_at = post.comments.get(&comment_id).unwrap().updated_at;

        // Add a like to comment
        let result = post.set_comment_like(comment_id.clone(), "user3".to_string(), LikeType::Like);

        assert!(result.is_ok());
        let comment = post.comments.get(&comment_id).unwrap();
        assert_eq!(comment.likes.len(), 1);
        assert_eq!(comment.likes.get("user3"), Some(&LikeType::Like));
        assert!(comment.updated_at > initial_updated_at);
    }

    #[test]
    fn test_set_comment_like_not_found() {
        let mut post = create_test_post();

        // Try to like non-existent comment
        let result = post.set_comment_like(
            "non-existent".to_string(),
            "user3".to_string(),
            LikeType::Like,
        );

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Comment not found");
    }

    #[test]
    fn test_remove_comment_like_success() {
        let mut post = create_test_post();
        let comment_id = post
            .add_comment("user2".to_string(), "Test comment".to_string(), None)
            .unwrap();

        // Add a like first
        post.set_comment_like(comment_id.clone(), "user3".to_string(), LikeType::Like)
            .unwrap();
        let comment = post.comments.get(&comment_id).unwrap();
        assert_eq!(comment.likes.len(), 1);

        let initial_updated_at = comment.updated_at;

        // Remove the like
        let result = post.remove_comment_like(comment_id.clone(), "user3".to_string());

        assert!(result.is_ok());
        let comment = post.comments.get(&comment_id).unwrap();
        assert_eq!(comment.likes.len(), 0);
        assert!(comment.updated_at > initial_updated_at);
    }

    #[test]
    fn test_remove_comment_like_not_found() {
        let mut post = create_test_post();
        let comment_id = post
            .add_comment("user2".to_string(), "Test comment".to_string(), None)
            .unwrap();
        let initial_updated_at = post.comments.get(&comment_id).unwrap().updated_at;

        // Try to remove like from non-existent comment
        let result1 = post.remove_comment_like("non-existent".to_string(), "user3".to_string());

        // Try to remove non-existent like from existing comment
        let result2 = post.remove_comment_like(comment_id.clone(), "user3".to_string());

        assert!(result1.is_err());
        assert_eq!(result1.unwrap_err(), "Comment not found");

        assert!(result2.is_ok()); // Function succeeds even if like didn't exist
        let comment = post.comments.get(&comment_id).unwrap();
        assert_eq!(comment.likes.len(), 0);
        assert_eq!(comment.updated_at, initial_updated_at); // Timestamp unchanged when no like removed
    }

    #[test]
    fn test_comment_new() {
        let comment = Comment::new(
            "user1".to_string(),
            "Test content".to_string(),
            Some("parent-id".to_string()),
        );

        assert!(!comment.comment_id.is_empty());
        assert_eq!(comment.content, "Test content");
        assert_eq!(comment.created_by, "user1");
        assert_eq!(comment.parent_comment_id, Some("parent-id".to_string()));
        assert!(comment.likes.is_empty());
        assert_eq!(comment.created_at, comment.updated_at);

        // Test that comment_id is a valid UUID
        uuid::Uuid::parse_str(&comment.comment_id).unwrap();
    }

    #[test]
    fn test_comment_new_no_parent() {
        let comment = Comment::new("user1".to_string(), "Test content".to_string(), None);

        assert!(!comment.comment_id.is_empty());
        assert_eq!(comment.content, "Test content");
        assert_eq!(comment.created_by, "user1");
        assert!(comment.parent_comment_id.is_none());
        assert!(comment.likes.is_empty());
        assert_eq!(comment.created_at, comment.updated_at);
    }

    #[test]
    fn test_post_like_operations_integration() {
        let mut post = create_test_post();

        // Add multiple likes
        assert!(!post.set_like("user2".to_string(), LikeType::Like));
        assert!(!post.set_like("user3".to_string(), LikeType::Love));
        assert!(!post.set_like("user4".to_string(), LikeType::Insightful));

        assert_eq!(post.likes.len(), 3);

        // Remove one like
        assert!(post.remove_like("user3".to_string()));

        assert_eq!(post.likes.len(), 2);
        assert_eq!(post.likes.get("user2"), Some(&LikeType::Like));
        assert_eq!(post.likes.get("user4"), Some(&LikeType::Insightful));
        assert!(post.likes.get("user3").is_none());

        // Override remaining like
        assert!(post.set_like("user2".to_string(), LikeType::Dislike));

        assert_eq!(post.likes.len(), 2);
        assert_eq!(post.likes.get("user2"), Some(&LikeType::Dislike));
        assert_eq!(post.likes.get("user4"), Some(&LikeType::Insightful));
    }

    #[test]
    fn test_comment_like_operations_integration() {
        let mut post = create_test_post();
        let comment_id = post
            .add_comment("user2".to_string(), "Test comment".to_string(), None)
            .unwrap();

        // Add multiple likes to comment
        assert!(post
            .set_comment_like(comment_id.clone(), "user3".to_string(), LikeType::Like)
            .is_ok());
        assert!(post
            .set_comment_like(comment_id.clone(), "user4".to_string(), LikeType::Love)
            .is_ok());
        assert!(post
            .set_comment_like(
                comment_id.clone(),
                "user5".to_string(),
                LikeType::Insightful
            )
            .is_ok());

        let comment = post.comments.get(&comment_id).unwrap();
        assert_eq!(comment.likes.len(), 3);

        // Remove one like
        assert!(post
            .remove_comment_like(comment_id.clone(), "user4".to_string())
            .is_ok());

        let comment = post.comments.get(&comment_id).unwrap();
        assert_eq!(comment.likes.len(), 2);
        assert_eq!(comment.likes.get("user3"), Some(&LikeType::Like));
        assert_eq!(comment.likes.get("user5"), Some(&LikeType::Insightful));
        assert!(comment.likes.get("user4").is_none());

        // Override remaining like
        assert!(post
            .set_comment_like(comment_id.clone(), "user3".to_string(), LikeType::Dislike)
            .is_ok());

        let comment = post.comments.get(&comment_id).unwrap();
        assert_eq!(comment.likes.len(), 2);
        assert_eq!(comment.likes.get("user3"), Some(&LikeType::Dislike));
        assert_eq!(comment.likes.get("user5"), Some(&LikeType::Insightful));
    }

    #[test]
    fn test_all_post_like_types() {
        let mut post = create_test_post();

        let like_types = vec![
            LikeType::Like,
            LikeType::Love,
            LikeType::Insightful,
            LikeType::Dislike,
        ];

        for (i, like_type) in like_types.iter().enumerate() {
            let user_id = format!("user{}", i + 2);
            assert!(!post.set_like(user_id, like_type.clone()));
        }

        assert_eq!(post.likes.len(), 4);
        assert_eq!(post.likes.get("user2"), Some(&LikeType::Like));
        assert_eq!(post.likes.get("user3"), Some(&LikeType::Love));
        assert_eq!(post.likes.get("user4"), Some(&LikeType::Insightful));
        assert_eq!(post.likes.get("user5"), Some(&LikeType::Dislike));
    }

    #[test]
    fn test_all_comment_like_types() {
        let mut post = create_test_post();
        let comment_id = post
            .add_comment("user2".to_string(), "Test comment".to_string(), None)
            .unwrap();

        let like_types = vec![
            LikeType::Like,
            LikeType::Love,
            LikeType::Insightful,
            LikeType::Dislike,
        ];

        for (i, like_type) in like_types.iter().enumerate() {
            let user_id = format!("user{}", i + 3);
            assert!(post
                .set_comment_like(comment_id.clone(), user_id, like_type.clone())
                .is_ok());
        }

        let comment = post.comments.get(&comment_id).unwrap();
        assert_eq!(comment.likes.len(), 4);
        assert_eq!(comment.likes.get("user3"), Some(&LikeType::Like));
        assert_eq!(comment.likes.get("user4"), Some(&LikeType::Love));
        assert_eq!(comment.likes.get("user5"), Some(&LikeType::Insightful));
        assert_eq!(comment.likes.get("user6"), Some(&LikeType::Dislike));
    }

    #[test]
    fn test_complex_comment_hierarchy() {
        let mut post = create_test_post();

        // Create a complex hierarchy:
        // comment1
        // ├── comment2
        // │   └── comment4
        // └── comment3

        let comment1 = post
            .add_comment("user2".to_string(), "Comment 1".to_string(), None)
            .unwrap();
        let comment2 = post
            .add_comment(
                "user3".to_string(),
                "Comment 2".to_string(),
                Some(comment1.clone()),
            )
            .unwrap();
        let comment3 = post
            .add_comment(
                "user4".to_string(),
                "Comment 3".to_string(),
                Some(comment1.clone()),
            )
            .unwrap();
        let comment4 = post
            .add_comment(
                "user5".to_string(),
                "Comment 4".to_string(),
                Some(comment2.clone()),
            )
            .unwrap();

        assert_eq!(post.comments.len(), 4);

        // Remove comment2 (should also remove comment4)
        assert!(post.remove_comment(comment2.clone()).is_ok());

        assert_eq!(post.comments.len(), 2);
        assert!(post.comments.contains_key(&comment1));
        assert!(post.comments.contains_key(&comment3));
        assert!(!post.comments.contains_key(&comment2));
        assert!(!post.comments.contains_key(&comment4));

        // Remove comment1 (should also remove comment3)
        assert!(post.remove_comment(comment1.clone()).is_ok());

        assert_eq!(post.comments.len(), 0);
    }

    #[test]
    fn test_post_update_from() {
        let post = create_test_post();
        let update = PostUpdate::from(&post);

        assert_eq!(update.post_id, post.post_id);
        assert_eq!(update.created_at, post.created_at);
        assert_eq!(update.updated_at, post.updated_at);
    }

    #[test]
    fn test_post_updates_new() {
        let updates = PostUpdates::new("user1".to_string());

        assert_eq!(updates.user_id, "user1");
        assert!(updates.updates.is_empty());
        assert_eq!(updates.created_at, updates.updated_at);
    }
}
