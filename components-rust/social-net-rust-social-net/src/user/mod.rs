use crate::post::PostAgentClient;
use email_address::EmailAddress;
use golem_rust::{agent_definition, agent_implementation, Schema};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;

#[derive(Schema, Clone, Serialize, Deserialize)]
pub struct ConnectedUser {
    pub user_id: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Schema, Clone, Serialize, Deserialize)]
pub struct User {
    pub user_id: String,
    pub name: Option<String>,
    pub email: Option<String>,
    pub connected_users: HashMap<String, ConnectedUser>,
    pub posts: Vec<String>,
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

    fn connect_user(&mut self, user_id: String) -> Result<(), String>;

    fn disconnect_user(&mut self, user_id: String) -> Result<(), String>;
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
            state.name = name;
            state.updated_at = chrono::Utc::now();

            Ok(())
        })
    }

    fn set_email(&mut self, email: Option<String>) -> Result<(), String> {
        self.with_state(|state| {
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

    fn connect_user(&mut self, user_id: String) -> Result<(), String> {
        let state = self.get_state();

        if user_id != state.user_id && !state.connected_users.contains_key(&user_id) {
            state.connected_users.insert(
                user_id.clone(),
                ConnectedUser {
                    user_id: user_id.clone(),
                    created_at: chrono::Utc::now(),
                },
            );

            UserAgentClient::get(user_id.clone()).trigger_connect_user(state.user_id.clone());
        }
        Ok(())
    }

    fn disconnect_user(&mut self, user_id: String) -> Result<(), String> {
        let state = self.get_state();

        if user_id != state.user_id && state.connected_users.contains_key(&user_id) {
            state.connected_users.remove(&user_id);

            UserAgentClient::get(user_id.clone()).trigger_disconnect_user(state.user_id.clone());
        }
        Ok(())
    }

    fn create_post(&mut self, content: String) -> Result<String, String> {
        let state = self.get_state();

        let post_id = uuid::Uuid::new_v4().to_string();

        PostAgentClient::get(post_id.clone()).trigger_init_post(state.user_id.clone(), content);

        state.posts.push(post_id.clone());

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
