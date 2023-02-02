run-dev: dev-dependencies generate
		cargo run -p jukebox_axum


dev-dependencies:
		rustup target add wasm32-unknown-unknown
		cargo install --locked trunk

generate:
		cd jukebox_yew && trunk build
