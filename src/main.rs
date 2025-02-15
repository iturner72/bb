use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(feature = "ssr")] {
        use axum::{
            body::Body as AxumBody,
            extract::State,
            http::Request,
            response::IntoResponse,
            routing::get,
            middleware,
            Router,
        };
        use dotenv::dotenv;
        use env_logger::Env;
        use leptos::prelude::*;
        use leptos_axum::{generate_route_list, handle_server_fns_with_context, LeptosRoutes};
        use bb::app::*;
        use bb::auth::server::middleware::require_auth;
        use bb::state::AppState;
        use bb::handlers::*;
        use bb::cancellable_sse::*;

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
            let _ = std::env::var("ADMIN_PASSWORD_HASH")
                .expect("ADMIN_PASSWORD_HASH must be set");

            let app_state = AppState {
                leptos_options: leptos_options.clone(),
                sse_state: SseState::new(),
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

            let protected_routes = Router::new()
                .route("/api/create-stream", get(create_stream))
                .route("/api/cancel-stream", get(cancel_stream))
                .route("/api/rss-progress", get(rss_progress_handler))
                .route("/api/backfill-progress", get(backfill_progress_handler))
                .route("/api/refresh-summaries", get(refresh_summaries_handler))
                .route("/api/generate-embeddings", get(embeddings_handler))
                .layer(middleware::from_fn(require_auth));

            let app = Router::new()
                .route(
                    "/api/*fn_name",
                    get(server_fn_handler).post(server_fn_handler),
                )
                .merge(protected_routes)
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
