use async_openai::{
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestSystemMessage, ChatCompletionRequestUserMessage,
        CreateChatCompletionRequestArgs, ResponseFormat, ResponseFormatJsonSchema,
    },
    Client,
};
use eyre::Result;
use once_cell::sync::Lazy;
use schemars::JsonSchema;
use serde::Deserialize;

const SYSTEM_MESSAGE: &str = r#"You are an AI assistant specialized in content moderation.
Your task is to analyze tweets for policy compliance. You must:
1. Carefully read the provided policy.
2. Examine the given tweet.
3. Determine if the tweet adheres to or violates the policy.
4. Provide a brief explanation for your decision."#;

fn create_compliance_prompt(policy: &str, tweet: &str) -> String {
    format!(
        r#"
Policy: {policy}
Tweet to analyze:
{tweet}
Analyze the tweet above and determine if it complies with the given policy.
"#
    )
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PolicyComplianceResponse {
    pub is_compliant: bool,
    pub _explanation: String,
}

static RESPONSE_FORMAT: Lazy<ResponseFormat> = Lazy::new(|| ResponseFormat::JsonSchema {
    json_schema: ResponseFormatJsonSchema {
        description: None,
        name: "policy_compliance".into(),
        schema: Some(
            serde_json::to_value(schemars::schema_for!(PolicyComplianceResponse)).unwrap(),
        ),
        strict: Some(true),
    },
});

pub struct OpenAIClient {
    client: Client<OpenAIConfig>,
}

impl OpenAIClient {
    pub fn new(api_key: &str) -> Self {
        Self { client: Client::with_config(OpenAIConfig::new().with_api_key(api_key)) }
    }

    // TODO: replace return type with full response
    pub async fn is_tweet_safe(&self, tweet: &str, policy: &str) -> Result<bool> {
        let request = CreateChatCompletionRequestArgs::default()
            .model("gpt-4o")
            .messages([
                ChatCompletionRequestSystemMessage::from(SYSTEM_MESSAGE.to_string()).into(),
                ChatCompletionRequestUserMessage::from(create_compliance_prompt(policy, tweet))
                    .into(),
            ])
            .response_format(RESPONSE_FORMAT.clone())
            .temperature(0.0)
            .build()?;

        let res = self.client.chat().create(request).await?;
        let parsed_response: PolicyComplianceResponse = serde_json::from_str(
            &res.choices
                .first()
                .ok_or_else(|| eyre::eyre!("No choices returned from OpenAI"))?
                .message
                .content
                .clone()
                .ok_or_else(|| eyre::eyre!("Empty content in OpenAI response"))?,
        )?;
        log::info!("gpt-4o response: {:#?}", parsed_response);
        Ok(parsed_response.is_compliant)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::CONFIG;

    async fn test_is_tweet_safe(client: &OpenAIClient, tweet: &str, policy: &str, expected: bool) {
        let is_safe = client.is_tweet_safe(tweet, policy).await.unwrap();
        assert_eq!(is_safe, expected);
    }

    #[tokio::test]
    async fn oai_unsafe_test() {
        let client = OpenAIClient::new(&CONFIG.secrets.openai_api_key);

        test_is_tweet_safe(
            &client,
            "I am going to go rob a bank.",
            "Don't allow any criminal planning or criminal activity.",
            false,
        )
        .await;
    }

    #[tokio::test]
    async fn oai_safe_test() {
        let client = OpenAIClient::new(&CONFIG.secrets.openai_api_key);

        test_is_tweet_safe(
            &client,
            "I am going to cry.",
            "Don't allow any criminal planning or criminal activity.",
            true,
        )
        .await;
    }
}
