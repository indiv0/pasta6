.PHONY: all
all:
	cargo watch -s "make all_"

.PHONY: all_
all_:
	cargo check
	cargo test --target aarch64-unknown-linux-gnu
	cargo test
	cargo run --release

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
