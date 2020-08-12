# pastaaaaaa

pastaaaaaa (AKA pasta6) is a REST API for uploading arbitrary bytes.
It's a pastebin-alike.

Endpoints:
* GET `/health` always returns 200 OK
* POST `/upload` echoes the bytes of the request body with 200 OK
  * Request bodies larger than 16kb are rejected
