# pastaaaaaa

pastaaaaaa (AKA pasta6) is a REST API for uploading arbitrary bytes.
It's a pastebin-alike.

Quickstart:
```sh
docker run --rm -p 5433:5432 -e POSTGRES_USER=pastaaaaaaa -e POSTGRES_PASSWORD=pastaaaaaaa -e POSTGRES_DB=pastaaaaaaa postgres:12.3
cargo run
```

Dependencies:
* PostgreSQL to store pastes and their metadata

Endpoints:
* GET `/health` always returns 200 OK
* POST `/upload` echoes the bytes of the request body with 200 OK
  * Request bodies larger than 16kb are rejected
