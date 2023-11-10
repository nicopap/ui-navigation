check:
	cargo clippy --examples

run:
	cargo run --example ultimate_menu_navigation --features="cuicui_chirp cuicui_layout_bevy_ui/chirp"

pre-hook:
	cargo fmt --all -- --check
	cargo clippy --no-default-features -- --deny clippy::all -D warnings
	RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features
	cargo clippy --all-features -- --deny clippy::all -D warnings
	cargo test --all-features
	cargo clippy --all-features --features="cuicui_chirp cuicui_layout_bevy_ui/chirp" --example ultimate_menu_navigation
