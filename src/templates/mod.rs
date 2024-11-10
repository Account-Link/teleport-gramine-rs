use askama::Template;
use axum::response::{Html, IntoResponse, Response};
use http::StatusCode;

#[derive(Template)]
#[template(path = "modal.html")]
pub struct PolicyTemplate {
    pub policy: String,
    pub address: String,
    pub x_id: String,
    pub event_id: String,
    pub user_email: String,
}

pub struct HtmlTemplate<T>(pub T);

impl<T> IntoResponse for HtmlTemplate<T>
where
    T: Template,
{
    fn into_response(self) -> Response {
        match self.0.render() {
            Ok(html) => Html(html).into_response(),
            Err(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to render template. Error: {err}"),
            )
                .into_response(),
        }
    }
}
