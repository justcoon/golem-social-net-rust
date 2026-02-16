use crate::common::{query, UserConnectionType};
use email_address::EmailAddress;
use futures::future::join_all;
use golem_rust::bindings::golem::api::host::{
    resolve_component_id, AgentAllFilter, AgentAnyFilter, AgentNameFilter, AgentPropertyFilter,
    GetAgents, StringFilterComparator,
};
use golem_rust::golem_wasm::ComponentId;
use golem_rust::{agent_definition, agent_implementation, Schema};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::str::FromStr;

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
        if self.connection_types.insert(connection_type) {
            self.updated_at = chrono::Utc::now();
        }
    }

    fn remove_connection_type(&mut self, connection_type: &UserConnectionType) {
        if self.connection_types.remove(connection_type) {
            self.updated_at = chrono::Utc::now();
        }
    }

    fn has_connection_type(&self, connection_type: &UserConnectionType) -> bool {
        self.connection_types.contains(connection_type)
    }
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
    fn new(user_id: String) -> Self {
        let now = chrono::Utc::now();
        User {
            user_id,
            name: None,
            email: None,
            connected_users: HashMap::new(),
            created_at: now,
            updated_at: now,
        }
    }

    fn set_name(&mut self, name: Option<String>) {
        self.name = name;
        self.updated_at = chrono::Utc::now();
    }

    fn set_email(&mut self, email: Option<String>) -> Result<(), String> {
        // Validate email format if provided
        if let Some(ref email_str) = email {
            EmailAddress::from_str(email_str).map_err(|e| format!("Invalid email: {e}"))?;
        }
        self.email = email;
        self.updated_at = chrono::Utc::now();
        Ok(())
    }

    fn connect_user(&mut self, user_id: String, connection_type: UserConnectionType) -> bool {
        if user_id == self.user_id {
            false
        } else {
            let should_connect = self
                .connected_users
                .get(&user_id)
                .is_none_or(|c| !c.has_connection_type(&connection_type));

            if should_connect {
                self.connected_users
                    .entry(user_id.clone())
                    .and_modify(|u| u.add_connection_type(connection_type.clone()))
                    .or_insert(ConnectedUser::new(user_id.clone(), connection_type.clone()));
                self.updated_at = chrono::Utc::now();
            }

            should_connect
        }
    }

    fn disconnect_user(&mut self, user_id: String, connection_type: UserConnectionType) -> bool {
        if user_id == self.user_id {
            false
        } else {
            let should_disconnect = self
                .connected_users
                .get(&user_id)
                .is_some_and(|c| c.has_connection_type(&connection_type));

            if should_disconnect {
                if self
                    .connected_users
                    .get(&user_id)
                    .is_some_and(|c| c.connection_types.len() == 1)
                {
                    self.connected_users.remove(&user_id);
                } else {
                    self.connected_users
                        .entry(user_id.clone())
                        .and_modify(|u| u.remove_connection_type(&connection_type));
                }
                self.updated_at = chrono::Utc::now();
            }

            should_disconnect
        }
    }
}

#[agent_definition]
trait UserAgent {
    fn new(id: String) -> Self;

    fn get_user(&self) -> Option<User>;

    fn set_name(&mut self, name: Option<String>) -> Result<(), String>;

    fn set_email(&mut self, email: Option<String>) -> Result<(), String>;

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
            state.set_name(name);
            Ok(())
        })
    }

    fn set_email(&mut self, email: Option<String>) -> Result<(), String> {
        self.with_state(|state| {
            println!("set email: {}", email.clone().unwrap_or("N/A".to_string()));
            state.set_email(email)
        })
    }

    fn connect_user(
        &mut self,
        user_id: String,
        connection_type: UserConnectionType,
    ) -> Result<(), String> {
        let state = self.get_state();
        if state.connect_user(user_id.clone(), connection_type.clone()) {
            println!("connect user - id: {user_id}, type: {connection_type}");

            let opposite_connection_type = connection_type.get_opposite();
            UserAgentClient::get(user_id.clone())
                .trigger_connect_user(state.user_id.clone(), opposite_connection_type);
        } else {
            println!(
                "connect user - id: {user_id}, type: {connection_type} - connection already exists or invalid"
            );
        }
        Ok(())
    }

    fn disconnect_user(
        &mut self,
        user_id: String,
        connection_type: UserConnectionType,
    ) -> Result<(), String> {
        let state = self.get_state();
        if state.disconnect_user(user_id.clone(), connection_type.clone()) {
            println!("disconnect user - id: {user_id}, type: {connection_type}");

            let opposite_connection_type = connection_type.get_opposite();
            UserAgentClient::get(user_id.clone())
                .trigger_disconnect_user(state.user_id.clone(), opposite_connection_type);
        } else {
            println!(
                "disconnect user - id: {user_id}, type: {connection_type} - connection not found or invalid"
            );
        }
        Ok(())
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

#[derive(Clone, Debug)]
struct UserQueryMatcher {
    query: query::Query,
}

impl UserQueryMatcher {
    fn new(query: &str) -> Self {
        let q = query::Query::new(query);

        Self { query: q }
    }

    // Check if a user matches the query
    fn matches(&self, user: User) -> bool {
        // Check field filters first
        for (field, value) in self.query.field_filters.iter() {
            let matches = match field.to_lowercase().as_str() {
                "user-id" | "userid" => query::text_exact_matches(&user.user_id, value),
                "name" => query::opt_text_matches(user.name.clone(), value),
                "email" => query::opt_text_exact_matches(user.email.clone(), value),
                _ => false, // Unknown field
            };

            if !matches {
                return false;
            }
        }

        // If no terms to match, just check if field filters passed
        if self.query.terms.is_empty() {
            return true;
        }

        // Check search terms against all searchable fields
        for term in self.query.terms.iter() {
            let matches = query::text_matches(&user.user_id, term)
                || query::opt_text_matches(user.name.clone(), term)
                || query::opt_text_matches(user.email.clone(), term);

            if !matches {
                return false;
            }
        }

        true
    }
}

fn get_agent_filter() -> AgentAnyFilter {
    AgentAnyFilter {
        filters: vec![AgentAllFilter {
            filters: vec![AgentPropertyFilter::Name(AgentNameFilter {
                comparator: StringFilterComparator::StartsWith,
                value: "user-agent(".to_string(),
            })],
        }],
    }
}

fn get_user_agent_id(agent_name: &str) -> Option<String> {
    Regex::new(r#"user-agent\("([^)]+)"\)"#)
        .ok()?
        .captures(agent_name)
        .filter(|caps| caps.len() > 0)
        .map(|caps| caps[1].to_string())
}

async fn get_users(
    agent_ids: HashSet<String>,
    matcher: UserQueryMatcher,
) -> Result<Vec<User>, String> {
    let clients: Vec<UserAgentClient> = agent_ids
        .into_iter()
        .map(|agent_id| UserAgentClient::get(agent_id.to_string()))
        .collect();

    let tasks: Vec<_> = clients.iter().map(|client| client.get_user()).collect();

    let responses = join_all(tasks).await;

    let result: Vec<User> = responses
        .into_iter()
        .flatten()
        .filter(|p| matcher.matches(p.clone()))
        .collect();

    Ok(result)
}

#[agent_definition(mode = "ephemeral")]
trait UserSearchAgent {
    fn new() -> Self;

    async fn search(&self, query: String) -> Result<Vec<User>, String>;
}

struct UserSearchAgentImpl {
    component_id: Option<ComponentId>,
}

#[agent_implementation]
impl UserSearchAgent for UserSearchAgentImpl {
    fn new() -> Self {
        let component_id = resolve_component_id("social-net-rust:social-net");
        UserSearchAgentImpl { component_id }
    }

    async fn search(&self, query: String) -> Result<Vec<User>, String> {
        if let Some(component_id) = self.component_id {
            println!("searching for users - query: {}", query);

            let mut values: Vec<User> = Vec::new();
            let matcher = UserQueryMatcher::new(&query);

            let filter = get_agent_filter();

            let get_agents = GetAgents::new(component_id, Some(&filter), false);

            let mut processed_agent_ids: HashSet<String> = HashSet::new();

            while let Some(agents) = get_agents.get_next() {
                let agent_ids = agents
                    .iter()
                    .filter_map(|a| get_user_agent_id(a.agent_id.agent_id.as_str()))
                    .filter(|n| !processed_agent_ids.contains(n))
                    .collect::<HashSet<_>>();

                let users = get_users(agent_ids.clone(), matcher.clone()).await?;
                processed_agent_ids.extend(agent_ids);
                values.extend(users);
            }

            Ok(values)
        } else {
            Err("Component not found".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::UserConnectionType;

    fn create_test_user() -> User {
        User::new("test-user-1".to_string())
    }

    fn create_test_connected_user(
        user_id: &str,
        connection_type: UserConnectionType,
    ) -> ConnectedUser {
        ConnectedUser::new(user_id.to_string(), connection_type)
    }

    #[test]
    fn test_user_new() {
        let user = User::new("test-user".to_string());
        assert_eq!(user.user_id, "test-user");
        assert!(user.name.is_none());
        assert!(user.email.is_none());
        assert!(user.connected_users.is_empty());
        assert_eq!(user.created_at, user.updated_at);
    }

    #[test]
    fn test_set_name_some() {
        let mut user = create_test_user();
        let initial_updated_at = user.updated_at;

        user.set_name(Some("John Doe".to_string()));

        assert_eq!(user.name, Some("John Doe".to_string()));
        assert!(user.updated_at > initial_updated_at);
    }

    #[test]
    fn test_set_name_none() {
        let mut user = create_test_user();
        user.set_name(Some("John Doe".to_string()));
        let initial_updated_at = user.updated_at;

        // Add a small delay to ensure timestamp difference
        std::thread::sleep(std::time::Duration::from_millis(1));

        user.set_name(None);

        assert!(user.name.is_none());
        assert!(user.updated_at > initial_updated_at);
    }

    #[test]
    fn test_set_email_valid() {
        let mut user = create_test_user();
        let initial_updated_at = user.updated_at;

        let result = user.set_email(Some("john.doe@example.com".to_string()));

        assert!(result.is_ok());
        assert_eq!(user.email, Some("john.doe@example.com".to_string()));
        assert!(user.updated_at > initial_updated_at);
    }

    #[test]
    fn test_set_email_invalid() {
        let mut user = create_test_user();
        let initial_updated_at = user.updated_at;
        let original_email = user.email.clone();

        let result = user.set_email(Some("invalid-email".to_string()));

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid email"));
        assert_eq!(user.email, original_email);
        assert_eq!(user.updated_at, initial_updated_at); // Should not update on error
    }

    #[test]
    fn test_set_email_none() {
        let mut user = create_test_user();
        user.set_email(Some("john.doe@example.com".to_string()))
            .unwrap();
        let initial_updated_at = user.updated_at;

        // Add a small delay to ensure timestamp difference
        std::thread::sleep(std::time::Duration::from_millis(1));

        let result = user.set_email(None);

        assert!(result.is_ok());
        assert!(user.email.is_none());
        assert!(user.updated_at > initial_updated_at);
    }

    #[test]
    fn test_connect_user_success() {
        let mut user = create_test_user();
        let initial_updated_at = user.updated_at;

        let result = user.connect_user("user2".to_string(), UserConnectionType::Friend);

        assert!(result);
        assert_eq!(user.connected_users.len(), 1);

        let connected_user = user.connected_users.get("user2").unwrap();
        assert_eq!(connected_user.user_id, "user2");
        assert!(connected_user.has_connection_type(&UserConnectionType::Friend));
        assert!(user.updated_at > initial_updated_at);
    }

    #[test]
    fn test_connect_user_self() {
        let mut user = create_test_user();
        let initial_updated_at = user.updated_at;

        let result = user.connect_user("test-user-1".to_string(), UserConnectionType::Friend);

        assert!(!result);
        assert!(user.connected_users.is_empty());
        assert_eq!(user.updated_at, initial_updated_at);
    }

    #[test]
    fn test_connect_user_already_connected() {
        let mut user = create_test_user();
        user.connect_user("user2".to_string(), UserConnectionType::Friend);
        let initial_updated_at = user.updated_at;

        let result = user.connect_user("user2".to_string(), UserConnectionType::Friend);

        assert!(!result);
        assert_eq!(user.connected_users.len(), 1);
        assert_eq!(user.updated_at, initial_updated_at);
    }

    #[test]
    fn test_connect_user_different_connection_type() {
        let mut user = create_test_user();
        user.connect_user("user2".to_string(), UserConnectionType::Friend);
        let initial_updated_at = user.updated_at;

        let result = user.connect_user("user2".to_string(), UserConnectionType::Follower);

        assert!(result);
        assert_eq!(user.connected_users.len(), 1);

        let connected_user = user.connected_users.get("user2").unwrap();
        assert!(connected_user.has_connection_type(&UserConnectionType::Friend));
        assert!(connected_user.has_connection_type(&UserConnectionType::Follower));
        assert!(user.updated_at > initial_updated_at);
    }

    #[test]
    fn test_connect_user_multiple_users() {
        let mut user = create_test_user();

        let result1 = user.connect_user("user2".to_string(), UserConnectionType::Friend);
        let result2 = user.connect_user("user3".to_string(), UserConnectionType::Follower);
        let result3 = user.connect_user("user4".to_string(), UserConnectionType::Friend);

        assert!(result1);
        assert!(result2);
        assert!(result3);
        assert_eq!(user.connected_users.len(), 3);

        assert!(user
            .connected_users
            .get("user2")
            .unwrap()
            .has_connection_type(&UserConnectionType::Friend));
        assert!(user
            .connected_users
            .get("user3")
            .unwrap()
            .has_connection_type(&UserConnectionType::Follower));
        assert!(user
            .connected_users
            .get("user4")
            .unwrap()
            .has_connection_type(&UserConnectionType::Friend));
    }

    #[test]
    fn test_disconnect_user_success() {
        let mut user = create_test_user();
        user.connect_user("user2".to_string(), UserConnectionType::Friend);
        let initial_updated_at = user.updated_at;

        let result = user.disconnect_user("user2".to_string(), UserConnectionType::Friend);

        assert!(result);
        assert!(user.connected_users.is_empty());
        assert!(user.updated_at > initial_updated_at);
    }

    #[test]
    fn test_disconnect_user_self() {
        let mut user = create_test_user();
        let initial_updated_at = user.updated_at;

        let result = user.disconnect_user("test-user-1".to_string(), UserConnectionType::Friend);

        assert!(!result);
        assert!(user.connected_users.is_empty());
        assert_eq!(user.updated_at, initial_updated_at);
    }

    #[test]
    fn test_disconnect_user_not_connected() {
        let mut user = create_test_user();
        let initial_updated_at = user.updated_at;

        let result = user.disconnect_user("user2".to_string(), UserConnectionType::Friend);

        assert!(!result);
        assert!(user.connected_users.is_empty());
        assert_eq!(user.updated_at, initial_updated_at);
    }

    #[test]
    fn test_disconnect_user_wrong_connection_type() {
        let mut user = create_test_user();
        user.connect_user("user2".to_string(), UserConnectionType::Friend);
        let initial_updated_at = user.updated_at;

        let result = user.disconnect_user("user2".to_string(), UserConnectionType::Follower);

        assert!(!result);
        assert_eq!(user.connected_users.len(), 1);
        assert_eq!(user.updated_at, initial_updated_at);
    }

    #[test]
    fn test_disconnect_user_multiple_connection_types() {
        let mut user = create_test_user();
        user.connect_user("user2".to_string(), UserConnectionType::Friend);
        user.connect_user("user2".to_string(), UserConnectionType::Follower);
        assert_eq!(user.connected_users.len(), 1);

        let connected_user = user.connected_users.get("user2").unwrap();
        assert_eq!(connected_user.connection_types.len(), 2);

        let initial_updated_at = user.updated_at;

        // Remove only one connection type
        let result = user.disconnect_user("user2".to_string(), UserConnectionType::Friend);

        assert!(result);
        assert_eq!(user.connected_users.len(), 1);

        let connected_user = user.connected_users.get("user2").unwrap();
        assert!(!connected_user.has_connection_type(&UserConnectionType::Friend));
        assert!(connected_user.has_connection_type(&UserConnectionType::Follower));
        assert!(user.updated_at > initial_updated_at);
    }

    #[test]
    fn test_disconnect_user_remove_last_connection() {
        let mut user = create_test_user();
        user.connect_user("user2".to_string(), UserConnectionType::Friend);
        user.connect_user("user2".to_string(), UserConnectionType::Follower);

        // Remove first connection type
        assert!(user.disconnect_user("user2".to_string(), UserConnectionType::Friend));
        assert_eq!(user.connected_users.len(), 1);

        // Remove second connection type (should remove user completely)
        let initial_updated_at = user.updated_at;
        let result = user.disconnect_user("user2".to_string(), UserConnectionType::Follower);

        assert!(result);
        assert!(user.connected_users.is_empty());
        assert!(user.updated_at > initial_updated_at);
    }

    #[test]
    fn test_connect_disconnect_cycle() {
        let mut user = create_test_user();

        // Connect user
        assert!(user.connect_user("user2".to_string(), UserConnectionType::Friend));
        assert_eq!(user.connected_users.len(), 1);

        // Disconnect user
        assert!(user.disconnect_user("user2".to_string(), UserConnectionType::Friend));
        assert!(user.connected_users.is_empty());

        // Reconnect user
        assert!(user.connect_user("user2".to_string(), UserConnectionType::Follower));
        assert_eq!(user.connected_users.len(), 1);

        let connected_user = user.connected_users.get("user2").unwrap();
        assert!(connected_user.has_connection_type(&UserConnectionType::Follower));
        assert!(!connected_user.has_connection_type(&UserConnectionType::Friend));
    }

    #[test]
    fn test_connected_user_new() {
        let connected_user = create_test_connected_user("user2", UserConnectionType::Friend);

        assert_eq!(connected_user.user_id, "user2");
        assert!(connected_user.has_connection_type(&UserConnectionType::Friend));
        assert_eq!(connected_user.connection_types.len(), 1);
        assert_eq!(connected_user.created_at, connected_user.updated_at);
    }

    #[test]
    fn test_connected_user_add_connection_type() {
        let mut connected_user = create_test_connected_user("user2", UserConnectionType::Friend);
        let initial_updated_at = connected_user.updated_at;

        connected_user.add_connection_type(UserConnectionType::Follower);

        assert!(connected_user.has_connection_type(&UserConnectionType::Friend));
        assert!(connected_user.has_connection_type(&UserConnectionType::Follower));
        assert_eq!(connected_user.connection_types.len(), 2);
        assert!(connected_user.updated_at > initial_updated_at);
    }

    #[test]
    fn test_connected_user_add_duplicate_connection_type() {
        let mut connected_user = create_test_connected_user("user2", UserConnectionType::Friend);
        let initial_updated_at = connected_user.updated_at;

        connected_user.add_connection_type(UserConnectionType::Friend);

        assert!(connected_user.has_connection_type(&UserConnectionType::Friend));
        assert_eq!(connected_user.connection_types.len(), 1);
        assert_eq!(connected_user.updated_at, initial_updated_at);
    }

    #[test]
    fn test_connected_user_remove_connection_type() {
        let mut connected_user = create_test_connected_user("user2", UserConnectionType::Friend);
        connected_user.add_connection_type(UserConnectionType::Follower);
        assert_eq!(connected_user.connection_types.len(), 2);

        // Add a small delay to ensure timestamp difference
        std::thread::sleep(std::time::Duration::from_millis(1));

        let initial_updated_at = connected_user.updated_at;

        connected_user.remove_connection_type(&UserConnectionType::Friend);

        assert!(!connected_user.has_connection_type(&UserConnectionType::Friend));
        assert!(connected_user.has_connection_type(&UserConnectionType::Follower));
        assert_eq!(connected_user.connection_types.len(), 1);
        assert!(connected_user.updated_at > initial_updated_at);
    }

    #[test]
    fn test_connected_user_remove_nonexistent_connection_type() {
        let mut connected_user = create_test_connected_user("user2", UserConnectionType::Friend);
        let initial_updated_at = connected_user.updated_at;

        connected_user.remove_connection_type(&UserConnectionType::Follower);

        assert!(connected_user.has_connection_type(&UserConnectionType::Friend));
        assert_eq!(connected_user.connection_types.len(), 1);
        assert_eq!(connected_user.updated_at, initial_updated_at);
    }

    #[test]
    fn test_connected_user_has_connection_type() {
        let mut connected_user = create_test_connected_user("user2", UserConnectionType::Friend);

        assert!(connected_user.has_connection_type(&UserConnectionType::Friend));
        assert!(!connected_user.has_connection_type(&UserConnectionType::Follower));

        connected_user.add_connection_type(UserConnectionType::Follower);

        assert!(connected_user.has_connection_type(&UserConnectionType::Friend));
        assert!(connected_user.has_connection_type(&UserConnectionType::Follower));
    }

    #[test]
    fn test_all_connection_types() {
        let mut user = create_test_user();

        let connection_types = vec![UserConnectionType::Friend, UserConnectionType::Follower];

        for (i, connection_type) in connection_types.iter().enumerate() {
            let user_id = format!("user{}", i + 2);
            assert!(user.connect_user(user_id, connection_type.clone()));
        }

        assert_eq!(user.connected_users.len(), 2);
        assert!(user
            .connected_users
            .get("user2")
            .unwrap()
            .has_connection_type(&UserConnectionType::Friend));
        assert!(user
            .connected_users
            .get("user3")
            .unwrap()
            .has_connection_type(&UserConnectionType::Follower));
    }

    #[test]
    fn test_complex_connection_scenario() {
        let mut user = create_test_user();

        // Create complex connections
        assert!(user.connect_user("user2".to_string(), UserConnectionType::Friend));
        assert!(user.connect_user("user2".to_string(), UserConnectionType::Follower));
        assert!(user.connect_user("user3".to_string(), UserConnectionType::Friend));
        assert!(user.connect_user("user4".to_string(), UserConnectionType::Follower));

        assert_eq!(user.connected_users.len(), 3);

        let user2_connections = user.connected_users.get("user2").unwrap();
        assert_eq!(user2_connections.connection_types.len(), 2);

        let user3_connections = user.connected_users.get("user3").unwrap();
        assert_eq!(user3_connections.connection_types.len(), 1);

        let user4_connections = user.connected_users.get("user4").unwrap();
        assert_eq!(user4_connections.connection_types.len(), 1);

        // Remove some connections
        assert!(user.disconnect_user("user2".to_string(), UserConnectionType::Friend));
        assert_eq!(user.connected_users.len(), 3);

        let user2_connections = user.connected_users.get("user2").unwrap();
        assert_eq!(user2_connections.connection_types.len(), 1);
        assert!(!user2_connections.has_connection_type(&UserConnectionType::Friend));
        assert!(user2_connections.has_connection_type(&UserConnectionType::Follower));

        // Remove user completely
        assert!(user.disconnect_user("user3".to_string(), UserConnectionType::Friend));
        assert_eq!(user.connected_users.len(), 2);
        assert!(!user.connected_users.contains_key("user3"));
    }

    #[test]
    fn test_user_operations_integration() {
        let mut user = create_test_user();

        // Set user properties
        user.set_name(Some("John Doe".to_string()));
        user.set_email(Some("john.doe@example.com".to_string()))
            .unwrap();

        // Create connections
        assert!(user.connect_user("friend1".to_string(), UserConnectionType::Friend));
        assert!(user.connect_user("follower1".to_string(), UserConnectionType::Follower));
        assert!(user.connect_user("friend2".to_string(), UserConnectionType::Friend));

        assert_eq!(user.name, Some("John Doe".to_string()));
        assert_eq!(user.email, Some("john.doe@example.com".to_string()));
        assert_eq!(user.connected_users.len(), 3);

        // Modify connections
        assert!(user.connect_user("friend1".to_string(), UserConnectionType::Follower));
        assert!(user.disconnect_user("friend2".to_string(), UserConnectionType::Friend));

        assert_eq!(user.connected_users.len(), 2); // friend2 should be completely removed

        let friend1_connections = user.connected_users.get("friend1").unwrap();
        assert!(friend1_connections.has_connection_type(&UserConnectionType::Friend));
        assert!(friend1_connections.has_connection_type(&UserConnectionType::Follower));

        // Update user properties again
        user.set_name(Some("Jane Doe".to_string()));
        let _ = user.set_email(None);

        assert_eq!(user.name, Some("Jane Doe".to_string()));
        assert!(user.email.is_none());
        assert_eq!(user.connected_users.len(), 2);
    }
}
