# pastaaaaaa

pastaaaaaa (AKA pasta6) is a REST API for uploading arbitrary bytes.
It's a pastebin-alike.

Quickstart:
```sh
docker run --rm -p 5432:5432 -e POSTGRES_PASSWORD=password postgres:12.3
cargo run
```

Dependencies:
* PostgreSQL to store pastes and their metadata

Endpoints:
* GET `/health` always returns 200 OK
* POST `/upload` echoes the bytes of the request body with 200 OK
  * Request bodies larger than 16kb are rejected
