use axum::{
    body::Body,
    extract::Path,
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::Response,
};

#[derive(Debug)]
pub(crate) struct EmbeddedAsset {
    path: &'static str,
    bytes: &'static [u8],
    content_type: &'static str,
}

include!(concat!(env!("OUT_DIR"), "/embedded_dashboard.rs"));

const CONTENT_SECURITY_POLICY: &str = "default-src 'self'; connect-src 'self' ws: wss:; img-src 'self' data:; style-src 'self'; script-src 'self'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'";

pub(crate) async fn root() -> Response<Body> {
    serve_path("index.html")
}

pub(crate) async fn path(Path(path): Path<String>) -> Response<Body> {
    if unsafe_path(&path) {
        return response(
            StatusCode::BAD_REQUEST,
            "text/plain; charset=utf-8",
            b"invalid path",
            false,
        );
    }
    if let Some(asset) = find_asset(&path) {
        return asset_response(asset);
    }
    if path == "api" || path.starts_with("api/") {
        return response(
            StatusCode::NOT_FOUND,
            "text/plain; charset=utf-8",
            b"API route not found",
            false,
        );
    }
    if !path
        .rsplit('/')
        .next()
        .is_some_and(|segment| segment.contains('.'))
    {
        return serve_path("index.html");
    }
    response(
        StatusCode::NOT_FOUND,
        "text/plain; charset=utf-8",
        b"asset not found",
        false,
    )
}

fn serve_path(path: &str) -> Response<Body> {
    find_asset(path).map_or_else(
        || {
            response(
                StatusCode::SERVICE_UNAVAILABLE,
                "text/plain; charset=utf-8",
                b"dashboard unavailable",
                false,
            )
        },
        asset_response,
    )
}

fn find_asset(path: &str) -> Option<&'static EmbeddedAsset> {
    EMBEDDED_ASSETS.iter().find(|asset| asset.path == path)
}

fn asset_response(asset: &'static EmbeddedAsset) -> Response<Body> {
    response(
        StatusCode::OK,
        asset.content_type,
        asset.bytes,
        asset.path != "index.html",
    )
}

fn response(
    status: StatusCode,
    content_type: &'static str,
    bytes: &'static [u8],
    immutable: bool,
) -> Response<Body> {
    let mut response = Response::new(Body::from(bytes));
    *response.status_mut() = status;
    let headers = response.headers_mut();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static(if immutable {
            "public, max-age=31536000, immutable"
        } else {
            "no-store"
        }),
    );
    security_headers(headers);
    response
}

fn security_headers(headers: &mut HeaderMap) {
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(CONTENT_SECURITY_POLICY),
    );
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer"),
    );
    headers.insert("x-frame-options", HeaderValue::from_static("DENY"));
    headers.insert(
        "permissions-policy",
        HeaderValue::from_static("camera=(), microphone=(), geolocation=()"),
    );
}

fn unsafe_path(path: &str) -> bool {
    path.contains('\\')
        || path.contains('\0')
        || path.split('/').any(|segment| matches!(segment, "." | ".."))
}
