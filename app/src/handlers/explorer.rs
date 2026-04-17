use axum::response::Html;

pub async fn explorer() -> Html<&'static str> {
    Html(include_str!("../explorer.html"))
}
