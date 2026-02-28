pub mod common {
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, Serialize, Deserialize)]
    #[serde(rename_all = "kebab-case")]
    pub enum UserConnectionType {
        Friend,
        Follower,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    #[serde(rename_all = "kebab-case")]
    pub enum LikeType {
        Like,
        Love,
        Insightful,
        Dislike,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    #[serde(rename_all = "kebab-case")]
    pub struct OkResult<T> {
        pub ok: T,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    #[serde(rename_all = "kebab-case")]
    pub struct ErrResult {
        pub err: ErrDetail,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    #[serde(rename_all = "kebab-case")]
    pub struct ErrDetail {
        pub message: String,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    #[serde(rename_all = "kebab-case")]
    pub struct PostCreated {
        pub post_id: String,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    #[serde(rename_all = "kebab-case")]
    pub struct CreatePost {
        pub content: String,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    #[serde(rename_all = "kebab-case")]
    pub struct CreateComment {
        pub content: String,
        pub user_id: String,
        pub parent_comment_id: Option<String>,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    #[serde(rename_all = "kebab-case")]
    pub struct SetLike {
        pub user_id: String,
        pub like_type: LikeType,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    #[serde(rename_all = "kebab-case")]
    pub struct CreateChat {
        pub participants: Vec<String>,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    #[serde(rename_all = "kebab-case")]
    pub struct ChatCreated {
        pub chat_id: String,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    #[serde(rename_all = "kebab-case")]
    pub struct AddMessage {
        pub user_id: String,
        pub content: String,
    }
}

pub mod social_net {
    use super::common::LikeType;
    use serde::{Deserialize, Serialize};
    use std::collections::{HashMap, HashSet};

    #[derive(Clone, Debug, Serialize, Deserialize)]
    #[serde(rename_all = "kebab-case")]
    pub struct User {
        pub user_id: String,
        pub name: Option<String>,
        pub email: Option<String>,
        pub created_at: String,
        pub updated_at: String,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    #[serde(rename_all = "kebab-case")]
    pub struct Post {
        pub post_id: String,
        pub content: String,
        pub created_by: String,
        pub likes: HashMap<String, LikeType>,
        pub created_at: String,
        pub updated_at: String,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    #[serde(rename_all = "kebab-case")]
    pub struct Chat {
        pub chat_id: String,
        pub created_by: String,
        pub participants: HashSet<String>,
        pub created_at: String,
        pub updated_at: String,
    }
}
