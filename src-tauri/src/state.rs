use serde::{Serialize, Deserialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

pub struct AppState {
    pub messages: Vec<Message>,
    pub abort_token: std::sync::Arc<tokio::sync::Notify>,
    pub listening: bool,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            abort_token: std::sync::Arc::new(tokio::sync::Notify::new()),
            listening: false,
        }
    }

    pub fn add_message(&mut self, role: &str, content: &str) {
        self.messages.push(Message {
            role: role.to_string(),
            content: content.to_string(),
        });

        // Keep last 10 messages
        if self.messages.len() > 20 {
            self.messages = self.messages.split_off(self.messages.len() - 20);
        }
    }

    pub fn get_context(&self, max: usize) -> Vec<Message> {
        let start = self.messages.len().saturating_sub(max);
        self.messages[start..].to_vec()
    }
}
