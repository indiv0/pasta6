# pastaaaaaa

pastaaaaaa (AKA pasta6) is a REST API for uploading arbitrary bytes.
It's a pastebin-alike.

## Quickstart

First, modify `default.conf` to point to your upstream address.
Replace `172.16.1.92` with your local dev machine's IP.

```
upstream pastaaaaaa {
    server 172.16.1.92:3030;
}
```

```sh
yarn install
yarn run tailwindcss build styles.css -o static/styles.css
docker run -d --name nginx --network host -v $(pwd)/static:/usr/share/nginx/html:ro -v $(pwd)/default.conf:/etc/nginx/conf.d/default.conf:ro docker run -d --name postgres --rm -p 5432:5432 -e POSTGRES_USER=pastaaaaaa -e POSTGRES_PASSWORD=pastaaaaaa -e POSTGRES_DB=pastaaaaaa postgres:12.3
export PG_HOST=localhost
export PG_USER=pastaaaaaa
export PG_PASSWORD=pastaaaaaa
cargo run
```

Dependencies:
* PostgreSQL to store pastes and their metadata

Endpoints:
* GET `/health` always returns 200 OK
* POST `/upload` echoes the bytes of the request body with 200 OK
  * Request bodies larger than 16kb are rejected

## Deploying

Assuming you have a EC2 t2.micro running Ubuntu 18.04.5 LTS, accessible at
`ubuntu@ec2-xxx-xxx-xxx-xxx.ca-central-1.compute.amazonaws.com`, you can follow
these steps to deploy pasta6 from scratch.

First, modify the EC2 instance's security group to allow tcp/80 traffic.

Compile & copy the `pastaaaaaa` binary to the instance:
```sh
yarn install
NODE_ENV=production yarn run tailwindcss build styles.css -o static/styles.css
cargo build --release
scp -i pastaaaaaa.pem target/release/pastaaaaaa ubuntu@ec2-xxx-xxx-xxx-xxx.ca-central-1.compute.amazonaws.com:
```

Login to the instance and install the dependencies:
```sh
# Login
ssh -i pastaaaaaa.pem ubuntu@ec2-xxx-xxx-xxx-xxx.ca-central-1.compute.amazonaws.com

# Install Nginx
sudo apt-get update
sudo apt-get upgrade
sudo apt-get install nginx

# Install PostgreSQL
sudo apt-get update
sudo apt-get upgrade
sudo apt-get install postgresql
```

Configure Nginx.
First, create the `/etc/nginx/sites-available/p.dank.xyz` file:
```sh
upstream pastaaaaaa {
    server localhost:3030;
}

server {
    listen       80;
    listen  [::]:80;
    server_name  p.dank.xyz;

    location / {
        proxy_pass         http://pastaaaaaa/;
        proxy_redirect     off;
        proxy_set_header   Host $host;
        proxy_set_header   X-Real-IP $remote_addr;
        proxy_set_header   X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header   X-Forwarded-Host $server_name;
    }

    location /styles.css {
        root   /srv/www/p.dank.xyz;
    }

    # redirect server error pages to the static page /50x.html
    #
    error_page   500 502 503 504  /50x.html;
    location = /50x.html {
        root   /usr/share/nginx/html;
    }
}
```
Link the site:
```sh
sudo ln -s /etc/nginx/sites-available/p.dank.xyz /etc/nginx/sites-enabled/
```
Remove the default site:
```sh
sudo rm /etc/nginx/sites-enabled/default
```
Test the config and reload Nginx:
```sh
sudo nginx -t
sudo nginx -s reload
```

Configure the database:
```sh
# Create a user & database for pasta6 to use
sudo -u postgres createuser --interactive # pastaaaaaa, y
sudo -u postgres createdb pastaaaaaa
sudo adduser pastaaaaaa
sudo -u pastaaaaaa psql # \password, \q

Run pasta6:
```sh
# Copy the static files to the directory served by nginx
sudo mkdir -p /srv/www/p.dank.xyz
sudo cp -r static/* /srv/www/p.dank.xyz/
sudo chown -R root:root /srv/www/p.dank.xyz

# Copy the pasta6 executable to the directory it will run in
sudo -u pastaaaaaa cp pastaaaaaa /home/pastaaaaaa/

# Run pasta6
sudo su pastaaaaaa
export PASTA6_HOST=127.0.0.1
export PG_HOST=localhost
export PG_USER=pastaaaaaa
export PG_PASSWORD=pastaaaaaa
sudo -u pastaaaaaa /home/pastaaaaaa/pastaaaaaa
```
