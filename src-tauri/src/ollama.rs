use reqwest::{Client, Response};
use serde::{Deserialize, Serialize};
use std::time::Duration;

const OLLAMA_BASE_URL: &str = "http://127.0.0.1:11434";
const MAX_MODEL_NAME_BYTES: usize = 128;
const MAX_MESSAGE_BYTES: usize = 16_384;
const MAX_CONVERSATION_BYTES: usize = 65_536;
const MAX_MESSAGES: usize = 40;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OllamaModel {
    pub name: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    User,
    Assistant,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
}

#[derive(Deserialize)]
struct ModelsResponse {
    models: Vec<OllamaModel>,
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: &'a [ChatMessage],
    stream: bool,
}

#[derive(Deserialize)]
struct ChatResponse {
    message: ChatMessage,
}

fn client() -> Result<Client, String> {
    Client::builder()
        .connect_timeout(Duration::from_secs(3))
        .timeout(Duration::from_secs(120))
        .no_proxy()
        .build()
        .map_err(|error| format!("Could not initialize the local Ollama client: {error}"))
}

fn connection_error(error: reqwest::Error) -> String {
    if error.is_connect() {
        format!(
            "Ollama is unreachable at {OLLAMA_BASE_URL}. Start the local Ollama service and retry."
        )
    } else {
        format!("Request to local Ollama failed: {error}")
    }
}

async fn check_response(response: Response) -> Result<Response, String> {
    let status = response.status();
    if status.is_success() {
        Ok(response)
    } else {
        Err(format!("Local Ollama returned HTTP {status}."))
    }
}

pub async fn list_models() -> Result<Vec<OllamaModel>, String> {
    let response = client()?
        .get(format!("{OLLAMA_BASE_URL}/api/tags"))
        .send()
        .await
        .map_err(connection_error)?;
    let response = check_response(response).await?;
    let mut models = response
        .json::<ModelsResponse>()
        .await
        .map_err(|error| format!("Could not read the model list from local Ollama: {error}"))?
        .models;
    models.sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));
    models.dedup_by(|left, right| left.name == right.name);
    Ok(models)
}

fn validate_chat(model: &str, messages: &[ChatMessage]) -> Result<(), String> {
    if model.trim().is_empty() || model.len() > MAX_MODEL_NAME_BYTES {
        return Err("Choose a valid local Ollama model.".to_string());
    }
    if messages.is_empty() || messages.len() > MAX_MESSAGES {
        return Err(format!(
            "A conversation must contain between 1 and {MAX_MESSAGES} messages."
        ));
    }

    let mut total_bytes = 0;
    for message in messages {
        if message.content.trim().is_empty() {
            return Err("Chat messages cannot be empty.".to_string());
        }
        if message.role == MessageRole::User && message.content.len() > MAX_MESSAGE_BYTES {
            return Err("A chat message is too long.".to_string());
        }
        total_bytes += message.content.len();
    }
    if total_bytes > MAX_CONVERSATION_BYTES {
        return Err("The conversation is too large to send to Ollama.".to_string());
    }
    if messages.last().map(|message| &message.role) != Some(&MessageRole::User) {
        return Err("The latest chat message must be from the user.".to_string());
    }
    Ok(())
}

pub async fn chat(model: &str, messages: &[ChatMessage]) -> Result<String, String> {
    validate_chat(model, messages)?;
    let response = client()?
        .post(format!("{OLLAMA_BASE_URL}/api/chat"))
        .json(&ChatRequest {
            model,
            messages,
            stream: false,
        })
        .send()
        .await
        .map_err(connection_error)?;
    let response = check_response(response).await?;
    let response = response
        .json::<ChatResponse>()
        .await
        .map_err(|error| format!("Could not read the response from local Ollama: {error}"))?;

    if response.message.role != MessageRole::Assistant {
        return Err("Local Ollama returned a response with an unexpected role.".to_string());
    }
    if response.message.content.trim().is_empty() {
        return Err("Local Ollama returned an empty response.".to_string());
    }
    Ok(response.message.content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_validation_requires_user_message_and_bounds_request_size() {
        let valid = vec![ChatMessage {
            role: MessageRole::User,
            content: "Hello".to_string(),
        }];
        assert!(validate_chat("llama3.2", &valid).is_ok());

        let assistant_last = vec![ChatMessage {
            role: MessageRole::Assistant,
            content: "Hello".to_string(),
        }];
        assert!(validate_chat("llama3.2", &assistant_last).is_err());

        let oversized = vec![ChatMessage {
            role: MessageRole::User,
            content: "x".repeat(MAX_MESSAGE_BYTES + 1),
        }];
        assert!(validate_chat("llama3.2", &oversized).is_err());
    }
}
