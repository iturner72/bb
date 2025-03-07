# BB Project Guidelines

## Build Commands
- Development server: `cargo leptos watch`
- Build for release: `cargo leptos build --release`
- End-to-end tests: `cargo leptos end-to-end`
- Single test embedding: `RUST_LOG=debug cargo run --bin test_embedding --features ssr`
- Hash password utility: `cargo run --bin hash_password --features ssr -- <password>`
- Download models: `cargo run --bin download_models --features ssr`
- Test local insert: `cargo run --bin test_local_insert --features ssr`
- Model inspector: `cargo run --bin model_inspector --features ssr`

## Code Style Guidelines
- **Rust Edition**: 2021
- **Framework**: Leptos 0.7.0 for frontend with Axum 0.7.5 for backend
- **Error Handling**: Use `thiserror` for error types
- **Naming**: Use snake_case for variables/functions, CamelCase for types
- **Components**: Place in `src/components/`, follow leptos component patterns
- **Server Functions**: Place in `src/server_fn/`
- **Modules**: Use proper module organization with mod.rs files
- **CSS**: Use Tailwind via the `style/tailwind.css` file
- **Formatting**: Use standard Rust formatting with `rustfmt`
- **Features**: Keep hydration (client) and SSR (server) dependencies separated properly

## Notes
- This is a Leptos web app with Axum backend and embedding services for semantic search
- Use correct feature flags when running binaries (`--features ssr`)