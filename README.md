# Jukebox Rust

Trying to create a Jukebox using Rust (axum + yew)

## Tools needed

Add wasm32 target via `rustup target add wasm32-unknown-unknown`
Install trunk via `cargo install --locked trunk` (cf. https://yew.rs/docs/getting-started/introduction)


## Run the server

Build the frontend using `trunk build` inside the jukebox_yew directory
Then run `cargo run -p jukebox_axum`
