use serde::Serialize;

#[derive(Debug, Serialize)]
struct Reply {
    in_reply_to_tweet_id: String,
}

#[derive(Debug, Serialize)]
struct Media {
    media_ids: Vec<String>,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Serialize, Default)]
pub struct Tweet {
    text: String,
    quote_tweet_id: Option<String>,
    reply: Option<Reply>,
    media: Option<Media>,
}

impl Tweet {
    pub fn new(text: String) -> Self {
        Self { text, quote_tweet_id: None, reply: None, media: None }
    }

    pub fn validate(&self) -> eyre::Result<()> {
        if self.text.is_empty() {
            eyre::bail!("Tweet text cannot be empty");
        }
        if self.quote_tweet_id.is_some() && self.reply.is_some() {
            eyre::bail!("Tweet cannot be both a quote and a reply");
        }
        if let Some(media) = &self.media {
            if media.media_ids.is_empty() {
                eyre::bail!("Media IDs cannot be empty");
            }
        }
        Ok(())
    }

    pub fn set_quote_tweet_id(&mut self, quote_tweet_id: String) {
        self.quote_tweet_id = Some(quote_tweet_id);
    }

    pub fn set_reply_tweet_id(&mut self, reply_tweet_id: String) {
        self.reply = Some(Reply { in_reply_to_tweet_id: reply_tweet_id });
    }

    pub fn set_media_ids(&mut self, media_ids: Vec<String>) {
        self.media = Some(Media { media_ids });
    }
}
