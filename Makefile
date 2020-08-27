watch:
	systemfd --no-pid -s http::0.0.0.0:3030 -- cargo watch -x run

styles:
	yarn run tailwindcss build styles.css -o static/styles.css

dependencies:
	docker run -d --rm --name nginx --network host -v $(PWD)/static:/usr/share/nginx/html:ro -v $(PWD)/default.conf:/etc/nginx/conf.d/default.conf:ro nginx:1.19.2
	docker run -d --rm --name postgres -p 5432:5432 -e POSTGRES_USER=$(POSTGRES_USER) -e POSTGRES_PASSWORD=$(POSTGRES_PASSWORD) -e POSTGRES_DB=$(POSTGRES_DB) postgres:12.3
