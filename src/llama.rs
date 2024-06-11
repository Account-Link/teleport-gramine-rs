use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};

#[derive(Debug, serde::Serialize)]
struct HuggingFaceParameters {
    return_full_text: Option<bool>,
}

#[derive(Debug, serde::Serialize)]
struct HuggingFaceInput {
    inputs: String,
    parameters: HuggingFaceParameters,
}

const TASK: &str = "Task: Check if there is unsafe content in \"User\" messages in conversations according our safety policy with the below categories.";
const OUTPUT: &str = "Provide your safety assessment for User in the above conversation: \n - First line must read 'safe' or 'unsafe'.\n - If unsafe, a second line must include a comma-separated list of violated categories.";

pub async fn is_tweet_safe(tweet: &String, policy: &String) -> bool {
    let client = reqwest::Client::new();
    let api_url = std::env::var("HUGGINGFACE_API_URL").expect("HUGGINGFACE_API_URL not set");
    let api_token = std::env::var("HUGGINGFACE_API_KEY").expect("HUGGINGFACE_API_KEY not set");
    let inputs = format!(
        "{}\n\n<BEGIN UNSAFE CONTENT CATEGORIES>\n\n{}\n<END UNSAFE CONTENT CATEGORIES>\n\n<BEGIN CONVERSATION>\nUser: {}\n<END CONVERSATION>\n\n{}\n",
        TASK, policy, tweet, OUTPUT
    );
    let parameters = HuggingFaceParameters {
        return_full_text: Some(false),
    };
    let llama_query = HuggingFaceInput { inputs, parameters };
    let input = serde_json::to_string(&llama_query).unwrap();
    let response = client
        .post(api_url)
        .header(AUTHORIZATION, format!("Bearer {}", api_token))
        .header(CONTENT_TYPE, "application/json")
        .body(input)
        .send()
        .await
        .unwrap();
    log::info!("{:?}", response.text().await);

    true
}

#[tokio::test]
async fn llama_safe_test() {
    env_logger::init();
    dotenv::dotenv().ok();
    let tweet_text = "I am going to go rob a bank.".to_string();
    let policy = "01: Criminal Planning".to_string();
    is_tweet_safe(&tweet_text, &policy).await;
}
