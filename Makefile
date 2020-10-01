.PHONY: watch_trigger watch_home watch_meta watch_paste styles dependencies nginx postgres release package deploy test

watch_trigger:
	cargo watch -i .trigger -x build -s "PASTA6_CONFIG=../config.toml cargo test --all" -s 'touch .trigger'

watch_home:
	systemfd --no-pid -s http::0.0.0.0:3030 -- cargo watch --no-gitignore -w .trigger -s "cargo run --package pasta6_home"

watch_meta:
	systemfd --no-pid -s http::0.0.0.0:3031 -- cargo watch --no-gitignore -w .trigger -s "cargo run --package pasta6_meta"

watch_paste:
	systemfd --no-pid -s http::0.0.0.0:3032 -- cargo watch --no-gitignore -w .trigger -s "cargo run --package pasta6_paste"

styles:
	yarn run tailwindcss build styles.css -o static/styles.css

dependencies: nginx postgres

nginx:
	docker run -d --rm --name nginx --network host \
		-v $(PWD)/static/styles.css:/usr/share/nginx/html/styles.css:ro \
		-v $(PWD)/static/robots.txt:/usr/share/nginx/html/robots.txt:ro \
		-v $(PWD)/nginx/default.conf:/etc/nginx/conf.d/default.conf:ro \
		-v $(PWD)/nginx/certs.conf:/etc/nginx/snippets/certs.conf:ro \
		-v $(PWD)/nginx/options-ssl-nginx.conf:/etc/nginx/snippets/options-ssl-nginx.conf:ro \
		-v /etc/letsencrypt/live/uh.rs/fullchain.pem:/etc/letsencrypt/live/uh.rs/fullchain.pem:ro \
		-v /etc/letsencrypt/live/uh.rs/chain.pem:/etc/letsencrypt/live/uh.rs/chain.pem:ro \
		-v /etc/letsencrypt/live/uh.rs/privkey.pem:/etc/letsencrypt/live/uh.rs/privkey.pem:ro \
		-v /etc/nginx/dhparam.pem:/etc/nginx/dhparam.pem:ro \
		nginx:1.19.2

postgres:
	docker run -d --rm --name postgres -p 5432:5432 \
		-e POSTGRES_USER=$(POSTGRES_USER) \
		-e POSTGRES_PASSWORD=$(POSTGRES_PASSWORD) \
		-e POSTGRES_DB=$(POSTGRES_DB) \
		-e POSTGRES_DBS=home.p6.rs,meta.p6.rs,paste.p6.rs \
		-v $(PWD)/init-postgres.sh:/docker-entrypoint-initdb.d/init-postgres.sh \
		postgres:12.3 postgres -c log_statement=all

postgres-cli:
	POSTGRES_PASSWORD=$(POSTGRES_PASSWORD) docker exec -it postgres psql --user $(POSTGRES_USER)

release:
	mkdir -p deploy/pasta6
	DOCKER_BUILDKIT=1 docker build . -t pasta6 --progress=plain
	docker create -ti --name dummy pasta6 bash
	docker cp dummy:/pasta6_home deploy/pasta6/pasta6_home
	docker cp dummy:/pasta6_meta deploy/pasta6/pasta6_meta
	docker cp dummy:/pasta6_paste deploy/pasta6/pasta6_paste
	docker cp dummy:/pasta6-generate-key deploy/pasta6/pasta6-generate-key
	docker rm dummy
	NODE_ENV=production yarn run tailwindcss build styles.css -o static/styles.css

package:
	mkdir -p deploy/pasta6/static deploy/pasta6/nginx
	cp certs-install.sh certs-renew.sh install.sh deploy/pasta6
	cp \
		static/styles.css \
		static/robots.txt \
		deploy/pasta6/static
	cp \
		nginx/certs.conf \
		nginx/default.conf \
		nginx/options-ssl-nginx.conf \
		deploy/pasta6/nginx
	(cd deploy && tar czvf pasta6.tar.gz pasta6)

deploy:
	scp deploy/pasta6.tar.gz pasta6:
	ssh pasta6 -- "sudo -u pasta6 "tar -C /home/pasta6 -xzvf /home/ubuntu/pasta6.tar.gz" && rm /home/ubuntu/pasta6.tar.gz"

test:
	cargo test
