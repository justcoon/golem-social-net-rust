use crate::common::LikeType;
use crate::user_chats::UserChatsAgentClient;
use futures::future::join_all;
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
                let removed = msg.likes.remove(&user_id).is_some();
                if removed {
                    let now = chrono::Utc::now();
                    msg.updated_at = now;
                    self.updated_at = now;
                }
                removed
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

pub async fn fetch_chats_by_ids(chat_ids: &[String]) -> Vec<Chat> {
    let mut result: Vec<Chat> = vec![];

    for chunk in chat_ids.chunks(10) {
        let clients = chunk
            .iter()
            .map(|chat_id| ChatAgentClient::get(chat_id.clone()))
            .collect::<Vec<_>>();

        let tasks: Vec<_> = clients.iter().map(|client| client.get_chat()).collect();
        let responses = join_all(tasks).await;

        let chunk_result: Vec<Chat> = responses.into_iter().flatten().collect();

        result.extend(chunk_result);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::LikeType;

    fn create_test_chat() -> Chat {
        let mut chat = Chat::new("test-chat-1".to_string());
        chat.created_by = "user1".to_string();
        chat.participants.insert("user1".to_string());
        chat.participants.insert("user2".to_string());
        chat
    }

    #[test]
    fn test_chat_new() {
        let chat = Chat::new("test-chat".to_string());
        assert_eq!(chat.chat_id, "test-chat");
        assert_eq!(chat.created_by, "");
        assert!(chat.participants.is_empty());
        assert!(chat.messages.is_empty());
        assert_eq!(chat.created_at, chat.updated_at);
    }

    #[test]
    fn test_add_message() {
        let mut chat = create_test_chat();
        let initial_updated_at = chat.updated_at;

        // Add first message
        let message_id1 = chat.add_message("user1".to_string(), "Hello world".to_string());

        assert_eq!(chat.messages.len(), 1);
        assert_eq!(chat.messages[0].message_id, message_id1);
        assert_eq!(chat.messages[0].content, "Hello world");
        assert_eq!(chat.messages[0].created_by, "user1");
        assert!(chat.messages[0].likes.is_empty());
        assert!(chat.updated_at > initial_updated_at);

        // Add second message
        let message_id2 = chat.add_message("user2".to_string(), "Hi there".to_string());

        assert_eq!(chat.messages.len(), 2);
        assert_eq!(chat.messages[1].message_id, message_id2);
        assert_eq!(chat.messages[1].content, "Hi there");
        assert_eq!(chat.messages[1].created_by, "user2");
        assert_ne!(message_id1, message_id2);
    }

    #[test]
    fn test_remove_message_success() {
        let mut chat = create_test_chat();
        let message_id = chat.add_message("user1".to_string(), "Test message".to_string());
        let initial_updated_at = chat.updated_at;

        // Remove existing message
        let result = chat.remove_message(message_id.clone());

        assert!(result);
        assert_eq!(chat.messages.len(), 0);
        assert!(chat.updated_at > initial_updated_at);
    }

    #[test]
    fn test_remove_message_not_found() {
        let mut chat = create_test_chat();
        let initial_updated_at = chat.updated_at;

        // Try to remove non-existent message
        let result = chat.remove_message("non-existent-id".to_string());

        assert!(!result);
        assert_eq!(chat.messages.len(), 0);
        assert_eq!(chat.updated_at, initial_updated_at);
    }

    #[test]
    fn test_remove_message_from_multiple() {
        let mut chat = create_test_chat();
        let message_id1 = chat.add_message("user1".to_string(), "Message 1".to_string());
        let message_id2 = chat.add_message("user2".to_string(), "Message 2".to_string());
        let message_id3 = chat.add_message("user1".to_string(), "Message 3".to_string());

        assert_eq!(chat.messages.len(), 3);

        // Remove middle message
        let result = chat.remove_message(message_id2.clone());

        assert!(result);
        assert_eq!(chat.messages.len(), 2);
        assert_eq!(chat.messages[0].message_id, message_id1);
        assert_eq!(chat.messages[1].message_id, message_id3);
    }

    #[test]
    fn test_set_message_like_success() {
        let mut chat = create_test_chat();
        let message_id = chat.add_message("user1".to_string(), "Test message".to_string());
        let initial_updated_at = chat.updated_at;

        // Add a like
        let result = chat.set_message_like(message_id.clone(), "user2".to_string(), LikeType::Like);

        assert!(result);
        assert_eq!(chat.messages[0].likes.len(), 1);
        assert_eq!(chat.messages[0].likes.get("user2"), Some(&LikeType::Like));
        assert!(chat.messages[0].updated_at > initial_updated_at);
        assert!(chat.updated_at > initial_updated_at);
    }

    #[test]
    fn test_set_message_like_not_found() {
        let mut chat = create_test_chat();
        let initial_updated_at = chat.updated_at;

        // Try to like non-existent message
        let result = chat.set_message_like(
            "non-existent-id".to_string(),
            "user2".to_string(),
            LikeType::Like,
        );

        assert!(!result);
        assert_eq!(chat.messages.len(), 0);
        assert_eq!(chat.updated_at, initial_updated_at);
    }

    #[test]
    fn test_set_multiple_likes() {
        let mut chat = create_test_chat();
        let message_id = chat.add_message("user1".to_string(), "Test message".to_string());

        // Add multiple likes from different users
        let result1 =
            chat.set_message_like(message_id.clone(), "user2".to_string(), LikeType::Like);
        let result2 =
            chat.set_message_like(message_id.clone(), "user3".to_string(), LikeType::Love);

        assert!(result1);
        assert!(result2);
        assert_eq!(chat.messages[0].likes.len(), 2);
        assert_eq!(chat.messages[0].likes.get("user2"), Some(&LikeType::Like));
        assert_eq!(chat.messages[0].likes.get("user3"), Some(&LikeType::Love));
    }

    #[test]
    fn test_override_like() {
        let mut chat = create_test_chat();
        let message_id = chat.add_message("user1".to_string(), "Test message".to_string());

        // Add initial like
        let result1 =
            chat.set_message_like(message_id.clone(), "user2".to_string(), LikeType::Like);

        // Override with different like type
        let result2 =
            chat.set_message_like(message_id.clone(), "user2".to_string(), LikeType::Love);

        assert!(result1);
        assert!(result2);
        assert_eq!(chat.messages[0].likes.len(), 1);
        assert_eq!(chat.messages[0].likes.get("user2"), Some(&LikeType::Love));
    }

    #[test]
    fn test_remove_message_like_success() {
        let mut chat = create_test_chat();
        let message_id = chat.add_message("user1".to_string(), "Test message".to_string());

        // Add a like first
        chat.set_message_like(message_id.clone(), "user2".to_string(), LikeType::Like);
        assert_eq!(chat.messages[0].likes.len(), 1);

        let initial_updated_at = chat.updated_at;

        // Remove the like
        let result = chat.remove_message_like(message_id.clone(), "user2".to_string());

        assert!(result);
        assert_eq!(chat.messages[0].likes.len(), 0);
        assert!(chat.messages[0].updated_at > initial_updated_at);
        assert!(chat.updated_at > initial_updated_at);
    }

    #[test]
    fn test_remove_message_like_not_found() {
        let mut chat = create_test_chat();
        let message_id = chat.add_message("user1".to_string(), "Test message".to_string());
        let initial_updated_at = chat.updated_at;

        // Try to remove like from non-existent message
        let result1 = chat.remove_message_like("non-existent-id".to_string(), "user2".to_string());

        // Try to remove non-existent like from existing message
        let result2 = chat.remove_message_like(message_id.clone(), "user2".to_string());

        assert!(!result1);
        assert!(!result2);
        assert_eq!(chat.messages[0].likes.len(), 0);
        assert_eq!(chat.updated_at, initial_updated_at);
    }

    #[test]
    fn test_message_new() {
        let message = Message::new("user1".to_string(), "Test content".to_string());

        assert!(!message.message_id.is_empty());
        assert_eq!(message.content, "Test content");
        assert_eq!(message.created_by, "user1");
        assert!(message.likes.is_empty());
        assert_eq!(message.created_at, message.updated_at);

        // Test that message_id is a valid UUID
        uuid::Uuid::parse_str(&message.message_id).unwrap();
    }

    #[test]
    fn test_like_operations_integration() {
        let mut chat = create_test_chat();
        let message_id = chat.add_message("user1".to_string(), "Test message".to_string());

        // Add multiple likes
        assert!(chat.set_message_like(message_id.clone(), "user2".to_string(), LikeType::Like));
        assert!(chat.set_message_like(message_id.clone(), "user3".to_string(), LikeType::Love));
        assert!(chat.set_message_like(
            message_id.clone(),
            "user4".to_string(),
            LikeType::Insightful
        ));

        assert_eq!(chat.messages[0].likes.len(), 3);

        // Remove one like
        assert!(chat.remove_message_like(message_id.clone(), "user3".to_string()));

        assert_eq!(chat.messages[0].likes.len(), 2);
        assert_eq!(chat.messages[0].likes.get("user2"), Some(&LikeType::Like));
        assert_eq!(
            chat.messages[0].likes.get("user4"),
            Some(&LikeType::Insightful)
        );
        assert!(chat.messages[0].likes.get("user3").is_none());

        // Override remaining like
        assert!(chat.set_message_like(message_id.clone(), "user2".to_string(), LikeType::Dislike));

        assert_eq!(chat.messages[0].likes.len(), 2);
        assert_eq!(
            chat.messages[0].likes.get("user2"),
            Some(&LikeType::Dislike)
        );
        assert_eq!(
            chat.messages[0].likes.get("user4"),
            Some(&LikeType::Insightful)
        );
    }

    #[test]
    fn test_all_like_types() {
        let mut chat = create_test_chat();
        let message_id = chat.add_message("user1".to_string(), "Test message".to_string());

        let like_types = vec![
            LikeType::Like,
            LikeType::Love,
            LikeType::Insightful,
            LikeType::Dislike,
        ];

        for (i, like_type) in like_types.iter().enumerate() {
            let user_id = format!("user{}", i + 2);
            assert!(chat.set_message_like(message_id.clone(), user_id, like_type.clone()));
        }

        assert_eq!(chat.messages[0].likes.len(), 4);
        assert_eq!(chat.messages[0].likes.get("user2"), Some(&LikeType::Like));
        assert_eq!(chat.messages[0].likes.get("user3"), Some(&LikeType::Love));
        assert_eq!(
            chat.messages[0].likes.get("user4"),
            Some(&LikeType::Insightful)
        );
        assert_eq!(
            chat.messages[0].likes.get("user5"),
            Some(&LikeType::Dislike)
        );
    }
}
