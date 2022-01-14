.PHONY: all
all:
	cargo watch -s "make all_"
	#cargo watch -s "cargo build --release --target=wasm32-wasi && cargo test --lib --target=wasm32-wasi && ~/.cargo/bin/lunatic target/wasm32-wasi/release/pasta6.wasm"
	#cargo watch -s "make configure && cargo test && cargo bench && cargo build --target=wasm32-wasi && lunaticvm pasta.wasm"

.PHONY: all_
all_:
	#cargo check && cargo wasi test -- --nocapture && cargo run --release
	#cargo check && cargo test --target aarch64-unknown-linux-gnu && cargo wasi test -- --nocapture
	#cargo check
	cargo test --target aarch64-unknown-linux-gnu
	#CARGO_TARGET_WASM32_WASI_RUNNER=lunatic cargo wasi test -- --nocapture
	cargo test

.PHONY: configure
configure:
	# Remove test database.
	sudo -u postgres psql -c "DROP DATABASE IF EXISTS test_db"
	# Create database user.
	sudo -u postgres psql -c "DROP ROLE IF EXISTS test_user; CREATE ROLE test_user WITH LOGIN PASSWORD 'test_password'"
	# Create database.
	sudo -u postgres createdb -O test_user test_db
	# Initialize database.
	PGPASSWORD=test_password psql -h 127.0.0.1 -U test_user -f sql/schema.sql test_db
	# Grant the user necessary permissions on the schema.
	PGPASSWORD=test_password psql -h 127.0.0.1 -U test_user -c "SELECT * FROM pasta.todo" test_db

.PHONY: bench
bench:
	./benchmark.sh

.PHONY: deploy
deploy:
	docker build . -t pasta6
	flyctl deploy --remote-only && flyctl info

.PHONY: psql
psql:
	sudo -u postgres psql
