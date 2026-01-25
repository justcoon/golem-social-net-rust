use crate::post::PostAgentClient;
use email_address::EmailAddress;
use golem_rust::{agent_definition, agent_implementation, Schema};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::str::FromStr;

#[derive(Schema, Clone, Serialize, Deserialize, Debug, Hash, Eq, PartialEq)]
pub enum UserConnectionType {
    Friend,
    Follower,
    Following,
}

impl UserConnectionType {
    fn get_opposite(&self) -> UserConnectionType {
        match self {
            UserConnectionType::Follower => UserConnectionType::Following,
            UserConnectionType::Following => UserConnectionType::Follower,
            UserConnectionType::Friend => UserConnectionType::Friend,
        }
    }
}

#[derive(Schema, Clone, Serialize, Deserialize)]
pub struct ConnectedUser {
    pub user_id: String,
    pub connection_types: HashSet<UserConnectionType>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl ConnectedUser {
    fn new(user_id: String, connection_type: UserConnectionType) -> Self {
        let now = chrono::Utc::now();
        ConnectedUser {
            user_id,
            connection_types: HashSet::from([connection_type]),
            created_at: now,
            updated_at: now,
        }
    }

    fn add_connection_type(&mut self, connection_type: UserConnectionType) {
        self.connection_types.insert(connection_type);
        self.updated_at = chrono::Utc::now();
    }

    fn remove_connection_type(&mut self, connection_type: &UserConnectionType) {
        self.connection_types.remove(&connection_type);
        self.updated_at = chrono::Utc::now();
    }

    fn has_connection_type(&self, connection_type: &UserConnectionType) -> bool {
        self.connection_types.contains(connection_type)
    }
}

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
pub struct User {
    pub user_id: String,
    pub name: Option<String>,
    pub email: Option<String>,
    pub connected_users: HashMap<String, ConnectedUser>,
    pub posts: Vec<PostRef>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl User {
    fn new(user_id: String) -> Self {
        let now = chrono::Utc::now();
        User {
            user_id,
            name: None,
            email: None,
            connected_users: HashMap::new(),
            posts: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }
}

#[agent_definition]
trait UserAgent {
    fn new(id: String) -> Self;

    fn get_user(&self) -> Option<User>;

    fn set_name(&mut self, name: Option<String>) -> Result<(), String>;

    fn set_email(&mut self, email: Option<String>) -> Result<(), String>;

    fn create_post(&mut self, content: String) -> Result<String, String>;

    fn connect_user(
        &mut self,
        user_id: String,
        connection_type: UserConnectionType,
    ) -> Result<(), String>;

    fn disconnect_user(
        &mut self,
        user_id: String,
        connection_type: UserConnectionType,
    ) -> Result<(), String>;
}

struct UserAgentImpl {
    _id: String,
    state: Option<User>,
}

impl UserAgentImpl {
    fn get_state(&mut self) -> &mut User {
        self.state.get_or_insert(User::new(self._id.clone()))
    }

    fn with_state<T>(&mut self, f: impl FnOnce(&mut User) -> T) -> T {
        f(self.get_state())
    }
}

#[agent_implementation]
impl UserAgent for UserAgentImpl {
    fn new(id: String) -> Self {
        UserAgentImpl {
            _id: id,
            state: None,
        }
    }

    fn get_user(&self) -> Option<User> {
        self.state.clone()
    }

    fn set_name(&mut self, name: Option<String>) -> Result<(), String> {
        self.with_state(|state| {
            println!("set name: {}", name.clone().unwrap_or("N/A".to_string()));
            state.name = name;
            state.updated_at = chrono::Utc::now();

            Ok(())
        })
    }

    fn set_email(&mut self, email: Option<String>) -> Result<(), String> {
        self.with_state(|state| {
            println!("set email: {}", email.clone().unwrap_or("N/A".to_string()));
            let _ = email
                .clone()
                .map(|email| {
                    EmailAddress::from_str(email.as_str())
                        .map_err(|e| format!("Invalid email: {e}"))
                })
                .transpose()?;
            state.email = email;
            state.updated_at = chrono::Utc::now();
            Ok(())
        })
    }

    fn connect_user(
        &mut self,
        user_id: String,
        connection_type: UserConnectionType,
    ) -> Result<(), String> {
        let state = self.get_state();
        if user_id == state.user_id {
            Err("Self connection not allowed".to_string())
        } else if state
            .connected_users
            .get(&user_id)
            .is_none_or(|c| !c.has_connection_type(&connection_type))
        {
            println!("connect user - id: {user_id}, type: {connection_type:?}");
            state
                .connected_users
                .entry(user_id.clone())
                .and_modify(|u| u.add_connection_type(connection_type.clone()))
                .or_insert(ConnectedUser::new(user_id.clone(), connection_type.clone()));

            let opposite_connection_type = connection_type.get_opposite();

            UserAgentClient::get(user_id.clone())
                .trigger_connect_user(state.user_id.clone(), opposite_connection_type);
            Ok(())
        } else {
            println!(
                "connect user - id: {user_id}, type: {connection_type:?} - connection already exists"
            );
            Ok(())
        }
    }

    fn disconnect_user(
        &mut self,
        user_id: String,
        connection_type: UserConnectionType,
    ) -> Result<(), String> {
        let state = self.get_state();
        if user_id == state.user_id {
            Err("Self connection not allowed".to_string())
        } else if state
            .connected_users
            .get(&user_id)
            .is_some_and(|c| c.has_connection_type(&connection_type))
        {
            println!("disconnect user - id: {user_id}, type: {connection_type:?}");
            if state
                .connected_users
                .get(&user_id)
                .is_some_and(|c| c.connection_types.len() == 1)
            {
                state.connected_users.remove(&user_id);
            } else {
                state
                    .connected_users
                    .entry(user_id.clone())
                    .and_modify(|u| u.remove_connection_type(&connection_type));
            }

            let opposite_connection_type = connection_type.get_opposite();

            UserAgentClient::get(user_id.clone())
                .trigger_disconnect_user(state.user_id.clone(), opposite_connection_type);
            Ok(())
        } else {
            println!(
                "disconnect user - id: {user_id}, type: {connection_type:?} - connection not found"
            );
            Ok(())
        }
    }

    fn create_post(&mut self, content: String) -> Result<String, String> {
        let state = self.get_state();

        let post_id = uuid::Uuid::new_v4().to_string();

        println!("create post - id: {post_id}");

        let post_ref = PostRef::new(post_id.clone());

        PostAgentClient::get(post_id.clone()).trigger_init_post(state.user_id.clone(), content);

        state.posts.push(post_ref);

        Ok(post_id)
    }

    async fn load_snapshot(&mut self, bytes: Vec<u8>) -> Result<(), String> {
        let data: Option<User> = crate::common::snapshot::deserialize(&bytes)?;
        self.state = data;
        Ok(())
    }

    async fn save_snapshot(&self) -> Result<Vec<u8>, String> {
        crate::common::snapshot::serialize(&self.state)
    }
}
