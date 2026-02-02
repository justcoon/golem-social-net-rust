use crate::common::LikeType;
use crate::user_chats::UserChatsAgentClient;
use golem_rust::{agent_definition, agent_implementation, Schema};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

const MAX_CHAT_LENGTH: usize = 2000;

#[derive(Schema, Clone, Serialize, Deserialize)]
pub struct Message {
    pub message_id: String,
    pub content: String,
    pub likes: HashMap<String, LikeType>,
    pub created_by: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl Message {
    fn new(user_id: String, content: String) -> Self {
        let now = chrono::Utc::now();
        let message_id = uuid::Uuid::new_v4().to_string();
        Message {
            message_id,
            content,
            likes: HashMap::new(),
            created_by: user_id,
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Schema, Clone, Serialize, Deserialize)]
pub struct Chat {
    pub chat_id: String,
    pub created_by: String,
    pub participants: HashSet<String>,
    pub messages: Vec<Message>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl Chat {
    fn new(chat_id: String) -> Self {
        let now = chrono::Utc::now();
        Chat {
            chat_id,
            messages: vec![],
            participants: HashSet::new(),
            created_by: "".to_string(),
            created_at: now,
            updated_at: now,
        }
    }

    fn add_message(&mut self, created_by: String, content: String) -> String {
        let message = Message::new(created_by.clone(), content);
        let message_id = message.message_id.clone();
        self.updated_at = message.created_at;
        self.messages.push(message);
        message_id
    }

    fn remove_message(&mut self, message_id: String) -> bool {
        if self.messages.iter().any(|m| m.message_id == message_id) {
            self.messages.retain(|m| m.message_id != message_id);
            self.updated_at = chrono::Utc::now();
            true
        } else {
            false
        }
    }

    fn set_message_like(
        &mut self,
        message_id: String,
        user_id: String,
        like_type: LikeType,
    ) -> bool {
        match self
            .messages
            .iter_mut()
            .find(|m| m.message_id == message_id)
        {
            Some(msg) => {
                msg.likes.insert(user_id, like_type);
                let now = chrono::Utc::now();
                msg.updated_at = now;
                self.updated_at = now;
                true
            }
            None => false,
        }
    }

    fn remove_message_like(&mut self, message_id: String, user_id: String) -> bool {
        match self
            .messages
            .iter_mut()
            .find(|m| m.message_id == message_id)
        {
            Some(msg) => {
                msg.likes.remove(&user_id);
                let now = chrono::Utc::now();
                msg.updated_at = now;
                self.updated_at = now;
                true
            }
            None => false,
        }
    }
}

#[agent_definition]
trait ChatAgent {
    fn new(id: String) -> Self;

    fn get_chat(&self) -> Option<Chat>;

    fn init_chat(
        &mut self,
        participants_ids: HashSet<String>,
        created_by: String,
        created_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), String>;

    fn add_participants(&mut self, participants_ids: HashSet<String>) -> Result<(), String>;

    fn add_message(&mut self, user_id: String, content: String) -> Result<String, String>;

    fn remove_message(&mut self, message_id: String) -> Result<(), String>;

    fn set_message_like(
        &mut self,
        message_id: String,
        user_id: String,
        like_type: LikeType,
    ) -> Result<(), String>;

    fn remove_message_like(&mut self, message_id: String, user_id: String) -> Result<(), String>;
}

struct ChatAgentImpl {
    _id: String,
    state: Option<Chat>,
}

impl ChatAgentImpl {
    fn get_state(&mut self) -> &mut Chat {
        self.state.get_or_insert(Chat::new(self._id.clone()))
    }

    fn with_state<T>(&mut self, f: impl FnOnce(&mut Chat) -> T) -> T {
        f(self.get_state())
    }
}

#[agent_implementation]
impl ChatAgent for ChatAgentImpl {
    fn new(id: String) -> Self {
        ChatAgentImpl {
            _id: id,
            state: None,
        }
    }

    fn get_chat(&self) -> Option<Chat> {
        self.state.clone()
    }

    fn init_chat(
        &mut self,
        participants_ids: HashSet<String>,
        created_by: String,
        created_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), String> {
        let mut participants_ids = participants_ids.clone();
        participants_ids.insert(created_by.clone());

        if self.state.is_some() {
            Err("Chat already exists".to_string())
        } else if participants_ids.len() < 2 {
            Err("Chat must have at least 2 participants".to_string())
        } else {
            let state = self.get_state();
            println!(
                "init chat - created by: {created_by}, participants: {}",
                participants_ids.len()
            );
            state.created_by = created_by.clone();
            state.participants.extend(participants_ids.clone());
            state.created_at = created_at;
            state.updated_at = created_at;

            execute_add_chat(
                state.chat_id.clone(),
                created_by.clone(),
                created_at,
                participants_ids,
            );

            Ok(())
        }
    }

    fn add_participants(&mut self, participants_ids: HashSet<String>) -> Result<(), String> {
        if self.state.is_none() {
            Err("Chat not exists".to_string())
        } else {
            self.with_state(|state| {
                let new_participants_ids: HashSet<String> = participants_ids
                    .into_iter()
                    .filter(|id| !state.participants.contains(id))
                    .collect();

                if new_participants_ids.is_empty() {
                    Err("No new participants".to_string())
                } else {
                    println!(
                        "add participants - new participants: {}",
                        new_participants_ids.len()
                    );
                    let old_participants_ids = state.participants.clone();

                    state.participants.extend(new_participants_ids.clone());
                    state.updated_at = chrono::Utc::now();

                    execute_add_chat(
                        state.chat_id.clone(),
                        state.created_by.clone(),
                        state.updated_at,
                        new_participants_ids,
                    );

                    execute_chat_updates(
                        state.chat_id.clone(),
                        old_participants_ids,
                        state.updated_at,
                    );
                    Ok(())
                }
            })
        }
    }

    fn add_message(&mut self, user_id: String, content: String) -> Result<String, String> {
        if self.state.is_none() {
            Err("Chat not exists".to_string())
        } else {
            self.with_state(|state| {
                println!("add message - user id: {}, content: {}", user_id, content);
                if state.messages.len() >= MAX_CHAT_LENGTH {
                    Err("Max chat length".to_string())
                } else {
                    let id = state.add_message(user_id.clone(), content);
                    execute_chat_updates(
                        state.chat_id.clone(),
                        state.participants.clone(),
                        state.updated_at,
                    );
                    Ok(id)
                }
            })
        }
    }

    fn remove_message(&mut self, message_id: String) -> Result<(), String> {
        if self.state.is_none() {
            Err("Chat not exists".to_string())
        } else {
            self.with_state(|state| {
                println!("remove message - message id: {}", message_id);
                if state.remove_message(message_id) {
                    execute_chat_updates(
                        state.chat_id.clone(),
                        state.participants.clone(),
                        state.updated_at,
                    );
                    Ok(())
                } else {
                    Err("Message not found".to_string())
                }
            })
        }
    }

    fn set_message_like(
        &mut self,
        message_id: String,
        user_id: String,
        like_type: LikeType,
    ) -> Result<(), String> {
        if self.state.is_none() {
            Err("Chat not exists".to_string())
        } else {
            self.with_state(|state| {
                println!(
                    "set message like - message id: {}, user id: {}, like type: {}",
                    message_id, user_id, like_type
                );
                if state.set_message_like(message_id, user_id, like_type) {
                    execute_chat_updates(
                        state.chat_id.clone(),
                        state.participants.clone(),
                        state.updated_at,
                    );
                    Ok(())
                } else {
                    Err("Message not found".to_string())
                }
            })
        }
    }

    fn remove_message_like(&mut self, message_id: String, user_id: String) -> Result<(), String> {
        if self.state.is_none() {
            Err("Chat not exists".to_string())
        } else {
            self.with_state(|state| {
                println!(
                    "remove message like - chat id: {}, user id: {}",
                    message_id, user_id
                );
                if state.remove_message_like(message_id, user_id) {
                    execute_chat_updates(
                        state.chat_id.clone(),
                        state.participants.clone(),
                        state.updated_at,
                    );
                    Ok(())
                } else {
                    Err("Message not found".to_string())
                }
            })
        }
    }

    async fn load_snapshot(&mut self, bytes: Vec<u8>) -> Result<(), String> {
        let data: Option<Chat> = crate::common::snapshot::deserialize(&bytes)?;
        self.state = data;
        Ok(())
    }

    async fn save_snapshot(&self) -> Result<Vec<u8>, String> {
        crate::common::snapshot::serialize(&self.state)
    }
}

fn execute_chat_updates(
    chat_id: String,
    participants_ids: HashSet<String>,
    updated_at: chrono::DateTime<chrono::Utc>,
) {
    for p_id in participants_ids {
        UserChatsAgentClient::get(p_id.clone()).trigger_chat_updated(chat_id.clone(), updated_at);
    }
}

fn execute_add_chat(
    chat_id: String,
    created_by: String,
    created_at: chrono::DateTime<chrono::Utc>,
    participants_ids: HashSet<String>,
) {
    for p_id in participants_ids {
        if p_id != created_by {
            UserChatsAgentClient::get(p_id.clone()).trigger_add_chat(
                chat_id.clone(),
                created_by.clone(),
                created_at,
            );
        }
    }
}
