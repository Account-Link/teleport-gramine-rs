const TASK: &str = "Task: Check if the tweet adheres to the provided policy.";
const OUTPUT: &str = "Provide your safety assessment for the above tweet as one word: either \"safe\" or \"unsafe\".";

pub async fn is_tweet_safe(tweet: &String, policy: &String) -> bool {
    let client =
        openai_rust::Client::new(&std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not set"));

    let inputs = format!(
        "{}\n<BEGIN POLICY>\n{}\n<END POLICY>\n<BEGIN TWEET>\n{}\n<END TWEET>\n{}\n",
        TASK, policy, tweet, OUTPUT
    );
    let args = openai_rust::chat::ChatArguments::new(
        "gpt-4o",
        vec![openai_rust::chat::Message {
            role: "user".to_owned(),
            content: inputs,
        }],
    );
    let res = client
        .create_chat(args)
        .await
        .expect("Failed to create chat");
    let is_unsafe = res.choices[0].message.content.contains("unsafe");
    log::info!("gpt-4o response: {:?}", res.choices[0].message.content);
    !is_unsafe
}

#[cfg(test)]
mod tests {
    use crate::oai::is_tweet_safe;

    async fn test_is_tweet_safe(tweet: &str, policy: &str, expected: bool) {
        dotenv::dotenv().ok();
        let is_safe = is_tweet_safe(&tweet.to_string(), &policy.to_string()).await;
        assert_eq!(is_safe, expected);
    }

    #[tokio::test]
    async fn oai_unsafe_test() {
        test_is_tweet_safe(
            "I am going to go rob a bank.",
            "Don't allow any criminal planning or criminal activity.",
            false,
        )
        .await;
    }

    #[tokio::test]
    async fn oai_safe_test() {
        test_is_tweet_safe(
            "I am going to cry.",
            "Don't allow any criminal planning or criminal activity.",
            true,
        )
        .await;
    }
}
