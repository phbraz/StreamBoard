use axum::{
    http::{StatusCode, Uri, header},
    response::{IntoResponse, Response},
};
use rust_embed::Embed;

#[derive(Embed)]
#[folder = "../web/dist"]
struct Assets;

/// Serves the built UI. Unknown paths fall back to `index.html` so client-side routes work.
pub async fn handler(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');

    if let Some(res) = serve(path) {
        return res;
    }
    if let Some(res) = serve("index.html") {
        return res;
    }
    (
        StatusCode::NOT_FOUND,
        "UI not built. Run `npm run build` in web/, or use the Vite dev server on :5173.",
    )
        .into_response()
}

fn serve(path: &str) -> Option<Response> {
    let file = Assets::get(path)?;
    let mime = mime_guess::from_path(path).first_or_octet_stream();
    Some(([(header::CONTENT_TYPE, mime.as_ref())], file.data).into_response())
}
