use axum::{
    body::Bytes,
    extract::{Host, MatchedPath},
    handler::HandlerWithoutStateExt,
    http::{HeaderMap, Request, StatusCode, Uri},
    response::{Html, Redirect, Response},
    routing::get,
    BoxError,
    Router,
};
use axum_server::tls_rustls::RustlsConfig;
use askama_axum::Template;
use rustls::{crypto::CryptoProvider, ServerConfig};
use tower_http::{classify::ServerErrorsFailureClass, trace::TraceLayer};
use tracing::{info_span, Span};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use std::{net::SocketAddr, path::PathBuf, sync::Arc, time::Duration};
use meet::{handlers::{self}, AppState, P2pRoomService};


// Define the HTML template using Askama
#[derive(Template)]
#[template(path = "index.html")]
struct RootTemplate {}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
    .with(
        tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            // axum logs rejections from built-in extractors with the `axum::rejection`
            // target, at `TRACE` level. `axum::rejection=trace` enables showing those events
            format!(
                "{}=debug,tower_http=debug,axum::rejection=trace",
                env!("CARGO_CRATE_NAME")
            )
            .into()
        }),
    )
    .with(tracing_subscriber::fmt::layer())
    .init();

    let ports = Ports {
        http: 3000,
        https: 7878,
    };
    rustls::crypto::ring::default_provider().install_default().expect("Failed to install rustls crypto provider");
    // CryptoProvider::install_default();
    let config = RustlsConfig::from_pem_file(
        PathBuf::from("./")
            .join("certs")
            .join("cert"),
        PathBuf::from("./")
            .join("certs")
            .join("key"),
    )
    .await
    .unwrap();

 
    tokio::spawn(redirect_http_to_https(ports));

    let app_state = AppState {
        room_service: Arc::new(P2pRoomService::new())
    };
    tracing::info!("Starting...");
    let app = Router::new()
        .route("/", get(root))
        .route("/ws/:room/:name", get(handlers::ws_handler))
        .with_state(app_state)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|request: &Request<_>| {
                    // Log the matched route's path (with placeholders not filled in).
                    // Use request.uri() or OriginalUri if you want the real path.
                    let matched_path = request
                        .extensions()
                        .get::<MatchedPath>()
                        .map(MatchedPath::as_str);

                    info_span!(
                        "http_request",
                        method = ?request.method(),
                        matched_path,
                        some_other_field = tracing::field::Empty,
                    )
                })
                .on_request(|_request: &Request<_>, _span: &Span| {
                    // You can use `_span.record("some_other_field", value)` in one of these
                    // closures to attach a value to the initially empty field in the info_span
                    // created above.
                })
                .on_response(|_response: &Response, _latency: Duration, _span: &Span| {
                    // ...
                })
                .on_body_chunk(|_chunk: &Bytes, _latency: Duration, _span: &Span| {
                    // ...
                })
                .on_eos(
                    |_trailers: Option<&HeaderMap>, _stream_duration: Duration, _span: &Span| {
                        // ...
                    },
                )
                .on_failure(
                    |_error: ServerErrorsFailureClass, _latency: Duration, _span: &Span| {
                        // ...
                    },
                ),
        );
    // let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    // axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await.unwrap();

    // run https server
    let addr = SocketAddr::from(([0, 0, 0, 0], ports.https));
    tracing::debug!("listening on {}", addr);
    axum_server::bind_rustls(addr, config)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .unwrap();
}

// Handler to display tasks
async fn root() -> Html<String> {
    let template = RootTemplate {};
    Html(template.render().unwrap())
}

#[allow(dead_code)]
#[derive(Clone, Copy)]
struct Ports {
    http: u16,
    https: u16,
}

#[allow(dead_code)]
async fn redirect_http_to_https(ports: Ports) {
    // TODO was getting infinite redirect, so just host root on the http port. It won't work on that port anyway.

    // fn make_https(host: String, uri: Uri, ports: Ports) -> Result<Uri, BoxError> {
    //     let mut parts = uri.into_parts();

    //     parts.scheme = Some(axum::http::uri::Scheme::HTTPS);

    //     if parts.path_and_query.is_none() {
    //         parts.path_and_query = Some("/".parse().unwrap());
    //     }

    //     let https_host = host.replace(&ports.http.to_string(), &ports.https.to_string());
    //     parts.authority = Some(https_host.parse()?);

    //     Ok(Uri::from_parts(parts)?)
    // }

    // let redirect = move |Host(host): Host, uri: Uri| async move {
    //     match make_https(host, uri, ports) {
    //         Ok(uri) => Ok(Redirect::permanent(&uri.to_string())),
    //         Err(error) => {
    //             tracing::warn!(%error, "failed to convert URI to HTTPS");
    //             Err(StatusCode::BAD_REQUEST)
    //         }
    //     }
    // };

    let addr = SocketAddr::from(([0, 0, 0, 0], ports.http));
    // let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    // tracing::debug!("listening on {}", listener.local_addr().unwrap());
    // axum::serve(listener, redirect.into_make_service())
    //     .await
    //     .unwrap();


    let app = Router::new().route("/", get(root));

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}