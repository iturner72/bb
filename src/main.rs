use bb::backfill_service;
use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(feature = "ssr")] {
        use axum::{
            body::Body as AxumBody,
            extract::{Query, State},
            http::Request,
            response::IntoResponse,
            response::sse::{Event, Sse},
            routing::get,
            Router,
        };
        use futures::stream::Stream;
        use tokio::sync::mpsc as tokio_mpsc;
        use std::convert::Infallible;
        use std::pin::Pin;
        use std::task::{Context, Poll};
        use std::collections::HashMap;
        use dotenv::dotenv;
        use env_logger::Env;
        use leptos::prelude::*;
        use leptos_axum::{generate_route_list, handle_server_fns_with_context, LeptosRoutes};
        use bb::app::*;
        use bb::rss_service::server::process_feeds_with_progress;
        use bb::state::AppState;

        pub struct SseStream {
            pub receiver: tokio_mpsc::Receiver<Result<Event, Infallible>>,
        }

        impl Stream for SseStream {
            type Item = Result<Event, Infallible>;

            fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
                self.receiver.poll_recv(cx)
            }
        }

        async fn rss_progress_handler(
            Query(_params): Query<HashMap<String, String>>,
        ) -> Sse<SseStream> {
            let (tx, rx) = tokio_mpsc::channel(100);

            tokio::spawn(async move {
                if let Err(e) = process_feeds_with_progress(tx).await {
                    log::error!("Error processing feeds: {}", e);
                }
            });

            Sse::new(SseStream { receiver: rx })
        }

        async fn backfill_progress_handler(
            Query(_params): Query<HashMap<String, String>>,
        ) -> Sse<SseStream> {
            let (tx, rx) = tokio_mpsc::channel(100);

            tokio::spawn(async move {
                if let Err(e) = backfill_service::backfill::backfill_missing_data(tx).await {
                    log::error!("Error during backfill: {}", e);
                }
            });

            Sse::new(SseStream { receiver: rx })
        }

        #[tokio::main]
        async fn main() {
            dotenv().ok();
            env_logger::init_from_env(Env::default().default_filter_or("info"));

            let conf = get_configuration(None).unwrap();
            let leptos_options = conf.leptos_options;
            let addr = leptos_options.site_addr;
            let routes = generate_route_list(App);

            let _ = std::env::var("JWT_SECRET")
                .expect("JWT_SECRET must be set");
            let _ = std::env::var("ADMIN_USERNAME")
                .expect("ADMIN_USERNAME must be set");
            let _ = std::env::var("ADMIN_PASSWORD")
                .expect("ADMIN_PASSWORD must be set");

            let app_state = AppState {
                leptos_options: leptos_options.clone(),
            };

            async fn server_fn_handler(
                State(app_state): State<AppState>,
                request: Request<AxumBody>,
            ) -> impl IntoResponse {
                handle_server_fns_with_context(
                    move || {
                        provide_context(app_state.clone());
                    },
                    request,
                )
                .await
            }

            let app = Router::new()
                .route(
                    "/api/*fn_name",
                    get(server_fn_handler).post(server_fn_handler),
                )
                .route("/api/rss-progress", get(rss_progress_handler))
                .route("/api/backfill-progress", get(backfill_progress_handler))
                .leptos_routes_with_handler(routes, get(|State(app_state): State<AppState>, request: Request<AxumBody>| async move {
                    let handler = leptos_axum::render_app_to_stream_with_context(
                        move || {
                            provide_context(app_state.clone());
                        },
                        move || shell(leptos_options.clone()) 
                    );
                    handler(request).await.into_response()
                }))
                .fallback(leptos_axum::file_and_error_handler::<AppState, _>(shell))
                .with_state(app_state);

            log::info!("Starting server at {}", addr);

            let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
            log::info!("listening on http://{}", &addr);
            axum::serve(listener, app.into_make_service()).await.unwrap();
        }
    } else {
        pub fn main() {
            // no client-side main function
            // unless we want this to work with e.g., Trunk for a purely client-side app
            // see lib.rs for hydration function instead
        }
    }
}
