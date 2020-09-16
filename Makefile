watch_meta:
	systemfd --no-pid -s http::0.0.0.0:3030 -- cargo watch -s "cargo run --package pasta6_meta"

watch_paste:
	systemfd --no-pid -s http::0.0.0.0:3031 -- cargo watch -s "cargo run --package pasta6_paste"

styles:
	yarn run tailwindcss build styles.css -o static/styles.css

dependencies:
	docker run -d --rm --name nginx --network host \
		-v $(PWD)/static:/usr/share/nginx/html:ro \
		-v $(PWD)/nginx/default.conf:/etc/nginx/conf.d/default.conf:ro \
		-v $(PWD)/nginx/certs.conf:/etc/nginx/snippets/certs.conf:ro \
		-v $(PWD)/nginx/options-ssl-nginx.conf:/etc/nginx/snippets/options-ssl-nginx.conf:ro \
		-v /etc/letsencrypt/live/uh.rs/fullchain.pem:/etc/letsencrypt/live/uh.rs/fullchain.pem:ro \
		-v /etc/letsencrypt/live/uh.rs/chain.pem:/etc/letsencrypt/live/uh.rs/chain.pem:ro \
		-v /etc/letsencrypt/live/uh.rs/privkey.pem:/etc/letsencrypt/live/uh.rs/privkey.pem:ro \
		-v /etc/nginx/dhparam.pem:/etc/nginx/dhparam.pem:ro \
		nginx:1.19.2
	docker run -d --rm --name postgres -p 5432:5432 -e POSTGRES_USER=$(POSTGRES_USER) -e POSTGRES_PASSWORD=$(POSTGRES_PASSWORD) -e POSTGRES_DB=$(POSTGRES_DB) postgres:12.3 postgres -c log_statement=all
