use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::Path;
use tokio::fs;
use base64::{Engine as _, engine::general_purpose};

#[derive(Clone)]
pub struct AIClient {
    client: Client,
    ollama_base_url: String,
    ollama_model: String,
    remote_base_url: String,
    remote_api_key: String,
    remote_model: String,
}

impl AIClient {
    pub fn new(
        ollama_base_url: String,
        ollama_model: String,
        remote_base_url: String,
        remote_api_key: String,
        remote_model: String,
    ) -> Self {
        Self {
            client: Client::new(),
            ollama_base_url,
            ollama_model,
            remote_base_url,
            remote_api_key,
            remote_model,
        }
    }

    pub async fn analyze_chart_vlm(&self, image_path: &Path, prompt: &str) -> Result<f64> {
        // Read image and encode to base64
        let image_data = fs::read(image_path).await?;
        let base64_image = general_purpose::STANDARD.encode(&image_data);

        // Construct Ollama request
        let request_body = json!({
            "model": self.ollama_model,
            "prompt": prompt,
            "images": [base64_image],
            "stream": false
        });

        let response = self.client
            .post(format!("{}/api/generate", self.ollama_base_url))
            .json(&request_body)
            .send()
            .await?;

        let response_text = response.text().await?;
        
        // Parse response to extract score (assuming JSON response from Ollama contains "response" field)
        // The prompt should instruct the model to return a JSON or single number.
        // For simplicity, we parse JSON if possible or look for a number.
        
        #[derive(Deserialize)]
        struct OllamaResponse {
            response: String,
        }
        
        let ollama_resp: OllamaResponse = serde_json::from_str(&response_text)?;
        
        // Naive parsing of score from text (expecting "0.85" or similar)
        // Real implementation would need robust parsing logic based on prompt instructions.
        let score = ollama_resp.response.trim().parse::<f64>().unwrap_or(0.0);
        
        Ok(score)
    }

    pub async fn query_llm(&self, prompt: &str) -> Result<String> {
        // OpenAI Compatible API
        let request_body = json!({
            "model": self.remote_model,
            "messages": [{"role": "user", "content": prompt}],
            "temperature": 0.7
        });

        let response = self.client
            .post(format!("{}/chat/completions", self.remote_base_url))
            .header("Authorization", format!("Bearer {}", self.remote_api_key))
            .json(&request_body)
            .send()
            .await?;

        let response_json: serde_json::Value = response.json().await?;
        
        let content = response_json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        Ok(content)
    }
}
