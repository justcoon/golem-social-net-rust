use crate::chat::{Chat, ChatAgentClient};
use crate::common::query;
use futures::future::join_all;
use golem_rust::{agent_definition, agent_implementation, Schema};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt::{Display, Formatter};
use std::{thread, time};

#[derive(Schema, Clone, Serialize, Deserialize)]
pub struct ChatRef {
    pub chat_id: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl ChatRef {
    fn new(chat_id: String) -> Self {
        let now = chrono::Utc::now();
        ChatRef {
            chat_id,
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Schema, Clone, Serialize, Deserialize)]
pub struct UserChats {
    pub user_id: String,
    pub chats: Vec<ChatRef>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl UserChats {
    fn new(user_id: String) -> Self {
        let now = chrono::Utc::now();
        UserChats {
            user_id,
            chats: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Schema, Clone, Serialize, Deserialize)]
pub struct UserChatsUpdates {
    pub user_id: String,
    pub chats: Vec<ChatRef>,
}

#[agent_definition]
trait UserChatsAgent {
    fn new(id: String) -> Self;

    fn get_chats(&self) -> Option<UserChats>;

    fn create_chat(&mut self, participants_ids: HashSet<String>) -> Result<String, String>;

    fn add_chat(
        &mut self,
        chat_id: String,
        created_by: String,
        created_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), String>;

    fn chat_updated(
        &mut self,
        chat_id: String,
        updated_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), String>;

    fn get_updates(&self, updates_since: chrono::DateTime<chrono::Utc>)
        -> Option<UserChatsUpdates>;
}

struct UserChatsAgentImpl {
    _id: String,
    state: Option<UserChats>,
}

impl UserChatsAgentImpl {
    fn get_state(&mut self) -> &mut UserChats {
        self.state.get_or_insert(UserChats::new(self._id.clone()))
    }

    fn with_state<T>(&mut self, f: impl FnOnce(&mut UserChats) -> T) -> T {
        f(self.get_state())
    }
}

#[agent_implementation]
impl UserChatsAgent for UserChatsAgentImpl {
    fn new(id: String) -> Self {
        UserChatsAgentImpl {
            _id: id,
            state: None,
        }
    }

    fn get_chats(&self) -> Option<UserChats> {
        self.state.clone()
    }

    fn create_chat(&mut self, participants_ids: HashSet<String>) -> Result<String, String> {
        self.with_state(|state| {
            let u_id = state.user_id.clone();
            let participants_ids: HashSet<String> = participants_ids
                .into_iter()
                .filter(|id| id.clone() != u_id)
                .collect::<HashSet<_>>();
            if participants_ids.is_empty() {
                Err("Chat must have at least 2 participants".to_string())
            } else {
                let chat_id = uuid::Uuid::new_v4().to_string();
                println!("create chat - id: {chat_id}");

                let chat_ref = ChatRef::new(chat_id.clone());
                let created_at = chat_ref.created_at;

                ChatAgentClient::get(chat_id.clone()).trigger_init_chat(
                    participants_ids,
                    state.user_id.clone(),
                    created_at,
                );

                state.chats.push(chat_ref);
                state.updated_at = created_at;

                Ok(chat_id)
            }
        })
    }

    fn add_chat(
        &mut self,
        chat_id: String,
        created_by: String,
        created_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), String> {
        self.with_state(|state| {
            let u_id = state.user_id.clone();
            if created_by == u_id {
                Err("Chat created by current user".to_string())
            } else {
                if !state.chats.iter().any(|c| c.chat_id == chat_id) {
                    println!("add chat - id: {chat_id}");

                    state.chats.push(ChatRef {
                        chat_id,
                        created_at,
                        updated_at: created_at,
                    });
                    if state.updated_at < created_at {
                        state.updated_at = created_at;
                    }
                }
                Ok(())
            }
        })
    }

    fn chat_updated(
        &mut self,
        chat_id: String,
        updated_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), String> {
        self.with_state(
            |state| match state.chats.iter_mut().find(|m| m.chat_id == chat_id) {
                Some(chat) => {
                    println!("chat updated - id: {chat_id}");
                    chat.updated_at = updated_at;
                    if state.updated_at < updated_at {
                        state.updated_at = updated_at;
                    }
                    Ok(())
                }
                None => Err("Chat not found".to_string()),
            },
        )
    }

    fn get_updates(
        &self,
        updates_since: chrono::DateTime<chrono::Utc>,
    ) -> Option<UserChatsUpdates> {
        if let Some(state) = &self.state {
            println!("get updates - updates since: {updates_since}");

            let updates = state
                .chats
                .iter()
                .filter(|p| p.updated_at > updates_since)
                .cloned()
                .collect();

            Some(UserChatsUpdates {
                user_id: state.user_id.clone(),
                chats: updates,
            })
        } else {
            None
        }
    }

    async fn load_snapshot(&mut self, bytes: Vec<u8>) -> Result<(), String> {
        let data: Option<UserChats> = crate::common::snapshot::deserialize(&bytes)?;
        self.state = data;
        Ok(())
    }

    async fn save_snapshot(&self) -> Result<Vec<u8>, String> {
        crate::common::snapshot::serialize(&self.state)
    }
}

#[derive(Clone, Debug)]
struct ChatQueryMatcher {
    terms: Vec<String>,
    field_filters: Vec<(String, String)>,
}

impl Display for ChatQueryMatcher {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ChatQueryMatcher(terms: {:?}, field_filters: {:?})",
            self.terms, self.field_filters
        )
    }
}

impl ChatQueryMatcher {
    fn new(query: &str) -> Self {
        let q = query::Query::new(query);

        Self {
            terms: q.terms,
            field_filters: q.field_filters,
        }
    }

    // Check if a chat matches the query
    fn matches_chat(&self, chat: Chat) -> bool {
        // Check field filters first
        for (field, value) in self.field_filters.iter() {
            let matches = match field.as_str() {
                "created-by" | "createdby" => query::text_exact_matches(&chat.created_by, value),
                "participants" => chat
                    .participants
                    .iter()
                    .any(|p| query::text_exact_matches(p, value)),
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
            let matches = query::text_matches(&chat.created_by, term)
                || chat
                    .participants
                    .iter()
                    .any(|p| query::text_exact_matches(p, term));

            if !matches {
                return false;
            }
        }

        true
    }
}

#[agent_definition(mode = "ephemeral")]
trait UserChatsViewAgent {
    fn new() -> Self;

    async fn get_chats_view(&mut self, user_id: String, query: String) -> Option<Vec<Chat>>;
}

struct UserChatsViewAgentImpl {}

#[agent_implementation]
impl UserChatsViewAgent for UserChatsViewAgentImpl {
    fn new() -> Self {
        Self {}
    }

    async fn get_chats_view(&mut self, user_id: String, query: String) -> Option<Vec<Chat>> {
        let user_chats = UserChatsAgentClient::get(user_id.clone()).get_chats().await;

        println!("get chats view - user id: {user_id}, query: {query}");

        if let Some(user_chats) = user_chats {
            let query_matcher = ChatQueryMatcher::new(&query);

            println!("get chats view - user id: {user_id}, query matcher: {query_matcher}");

            let user_chats = user_chats.chats;

            if user_chats.is_empty() {
                Some(vec![])
            } else {
                let mut result: Vec<Chat> = vec![];
                for chunk in user_chats.chunks(10) {
                    let clients = chunk
                        .iter()
                        .map(|p| ChatAgentClient::get(p.chat_id.clone()))
                        .collect::<Vec<_>>();

                    let tasks: Vec<_> = clients.iter().map(|client| client.get_chat()).collect();

                    let responses = join_all(tasks).await;

                    let chunk_result: Vec<Chat> = responses
                        .into_iter()
                        .flatten()
                        .filter(|p| query_matcher.matches_chat(p.clone()))
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

#[agent_definition(mode = "ephemeral")]
trait UserChatsUpdatesAgent {
    fn new() -> Self;

    async fn get_chats_updates(
        &mut self,
        user_id: String,
        updates_since: Option<chrono::DateTime<chrono::Utc>>,
        iter_wait_time: Option<u32>,
        max_wait_time: Option<u32>,
    ) -> Option<Vec<ChatRef>>;
}

struct UserChatsUpdatesAgentImpl {}

#[agent_implementation]
impl UserChatsUpdatesAgent for UserChatsUpdatesAgentImpl {
    fn new() -> Self {
        Self {}
    }

    async fn get_chats_updates(
        &mut self,
        user_id: String,
        updates_since: Option<chrono::DateTime<chrono::Utc>>,
        iter_wait_time: Option<u32>,
        max_wait_time: Option<u32>,
    ) -> Option<Vec<ChatRef>> {
        let since = updates_since.unwrap_or(chrono::Utc::now());
        let max_wait_time = time::Duration::from_millis(max_wait_time.unwrap_or(10000) as u64);
        let iter_wait_time = time::Duration::from_millis(iter_wait_time.unwrap_or(500) as u64);
        let now = time::Instant::now();
        let mut done = false;
        let mut result: Option<Vec<ChatRef>> = None;

        while !done {
            println!(
                "get chats updates - user id: {}, updates since: {}, elapsed time: {}ms, max wait time: {}ms",
                user_id,
                since,
                now.elapsed().as_millis(),
                max_wait_time.as_millis()
            );
            let res = UserChatsAgentClient::get(user_id.clone())
                .get_updates(since)
                .await;

            if let Some(updates) = res {
                if !updates.chats.is_empty() {
                    result = Some(updates.chats);
                    done = true;
                } else {
                    result = Some(vec![]);
                    done = now.elapsed() >= max_wait_time;
                    if !done {
                        thread::sleep(iter_wait_time);
                    }
                }
            } else {
                result = None;
                done = true;
            }
        }
        result
    }
}
