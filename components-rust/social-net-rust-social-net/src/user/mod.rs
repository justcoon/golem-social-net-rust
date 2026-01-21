use golem_rust::{agent_definition, agent_implementation, Schema};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl User {
    pub fn new(user_id: String) -> Self {
        User {
            user_id,
            name: None,
            email: None,
            connected_users: HashMap::new(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }
}

#[agent_definition]
trait UserAgent {
    fn new(id: String) -> Self;

    fn get_user(&self) -> Option<User>;

    fn set_name(&mut self, name: Option<String>);

    fn set_email(&mut self, email: Option<String>);

    async fn connect_user(&mut self, user_id: String);

    async fn disconnect_user(&mut self, user_id: String);
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

    fn set_name(&mut self, name: Option<String>) {
        self.with_state(|state| {
            state.name = name;
            state.updated_at = chrono::Utc::now();
        })
    }

    fn set_email(&mut self, email: Option<String>) {
        self.with_state(|state| {
            state.email = email;
            state.updated_at = chrono::Utc::now();
        })
    }

    async fn connect_user(&mut self, user_id: String) {
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
    }

    async fn disconnect_user(&mut self, user_id: String) {
        let state = self.get_state();

        if user_id != state.user_id && state.connected_users.contains_key(&user_id) {
            state.connected_users.remove(&user_id);

            UserAgentClient::get(user_id.clone()).trigger_disconnect_user(state.user_id.clone());
        }
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
