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
        self.connection_types.insert(connection_type);
        self.updated_at = chrono::Utc::now();
    }

    fn remove_connection_type(&mut self, connection_type: &UserConnectionType) {
        self.connection_types.remove(connection_type);
        self.updated_at = chrono::Utc::now();
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
            println!("connect user - id: {user_id}, type: {connection_type}");
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
                "connect user - id: {user_id}, type: {connection_type} - connection already exists"
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
            println!("disconnect user - id: {user_id}, type: {connection_type}");
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
                "disconnect user - id: {user_id}, type: {connection_type} - connection not found"
            );
            Ok(())
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
