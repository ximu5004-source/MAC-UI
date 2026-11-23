use crate::resources::RESOURCES;
use salvo::prelude::*;

/// Stable loopback port retained for existing local automation clients.
pub const LOCAL_API_PORT: u16 = 37_074;

const SCALAR_HTML: &str = include_str!("./scalar.html");

/// Serves the self-contained local API reference without external scripts.
#[handler]
async fn scalar_docs(res: &mut Response) {
    res.render(Text::Html(SCALAR_HTML));
}

#[derive(serde::Serialize, salvo::oapi::ToSchema)]
struct PingResponse {
    name: &'static str,
    version: &'static str,
}

/// Ping
///
/// Returns the name and version of the application.
#[endpoint]
async fn ping() -> Json<PingResponse> {
    Json(PingResponse {
        name: env!("CARGO_PKG_NAME"),
        version: env!("CARGO_PKG_VERSION"),
    })
}

/// Themes
///
/// Returns a list of all available themes.
#[endpoint(tag("Resources"))]
async fn themes() -> Json<Vec<std::sync::Arc<macui_core::state::Theme>>> {
    Json(RESOURCES.themes())
}

/// Icon Packs
///
/// Returns a list of all available icon packs.
#[endpoint(tag("Resources"))]
async fn icon_packs() -> Json<Vec<std::sync::Arc<macui_core::state::IconPack>>> {
    Json(RESOURCES.icon_packs())
}

/// Theme Tokens
///
/// Returns the design tokens resolved from all currently enabled themes.
/// Enabled themes are merged in activation order, so themes activated later
/// take priority over the ones activated before them.
#[endpoint(tag("Resources"))]
async fn theme_tokens(
    // Whether to resolve dark-mode tokens. Themes without `tokensDark` fall back to `tokens`.
    dark: salvo::oapi::extract::QueryParam<bool, false>,
) -> Json<macui_core::state::ThemeTokens> {
    let dark = dark.into_inner().unwrap_or(false);
    let state = crate::state::application::FULL_STATE.load();

    let themes_by_id: std::collections::HashMap<_, _> = RESOURCES
        .themes()
        .into_iter()
        .map(|theme| (theme.id.clone(), theme))
        .collect();

    let tokens = state
        .settings
        .active_themes
        .iter()
        .filter_map(|id| themes_by_id.get(id))
        .filter_map(|theme| {
            if dark {
                theme.tokens_dark.as_ref().or(theme.tokens.as_ref())
            } else {
                theme.tokens.as_ref()
            }
        })
        .cloned()
        .fold(macui_core::state::ThemeTokens::default(), |acc, tokens| {
            acc.merge(tokens)
        });

    Json(tokens)
}

/* #[endpoint]
async fn settings() -> Json<macui_core::state::Settings> {
    let state = crate::state::application::FULL_STATE.load();
    Json(&state.settings)
} */

/// Starts the background HTTP server on its own thread, with a dedicated single-threaded
/// runtime, so it never competes with the main runtime workers. Intended to be called once
/// at app startup.
pub fn start_http_server() {
    let spawned = std::thread::Builder::new()
        .name("http-server".into())
        .spawn(|| {
            let runtime = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(runtime) => runtime,
                Err(err) => {
                    log::error!("Failed to create HTTP server runtime: {err:?}");
                    return;
                }
            };
            runtime.block_on(run_http_server());
        });

    if let Err(err) = spawned {
        log::error!("Failed to spawn HTTP server thread: {err:?}");
    }
}

async fn run_http_server() {
    let api = Router::new()
        .push(Router::with_path("ping").get(ping))
        .push(
            Router::with_path("resources")
                .push(Router::with_path("themes").get(themes))
                .push(Router::with_path("themes/tokens").get(theme_tokens))
                .push(Router::with_path("icon-packs").get(icon_packs)),
        );

    let doc = OpenApi::new("MAC UI Local API", env!("CARGO_PKG_VERSION")).merge_router(&api);

    let router = Router::new()
        .push(api)
        .push(doc.into_router("/api-doc/openapi.json"))
        .push(Router::with_path("api-doc").goal(scalar_docs));

    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], LOCAL_API_PORT));
    let acceptor = match TcpListener::new(addr).try_bind().await {
        Ok(acceptor) => acceptor,
        Err(err) => {
            log::error!("Failed to bind HTTP server on {addr}: {err:?}");
            return;
        }
    };

    log::info!("HTTP server listening on {addr}");
    Server::new(acceptor).serve(router).await;
}
