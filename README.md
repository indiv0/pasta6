# pastaaaaaa

pastaaaaaa (AKA pasta6) is a REST API for uploading arbitrary bytes.
It's a pastebin-alike.

## Quickstart

```sh
docker run --name postgres --rm -p 5433:5432 -e POSTGRES_USER=pastaaaaaa -e POSTGRES_PASSWORD=pastaaaaaa -e POSTGRES_DB=pastaaaaaa postgres:12.3
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
server {
    listen 80;
    listen [::]:80;

    server_name p.dank.xyz;

    location / {
        proxy_pass http://localhost:3030;
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
# Copy the pasta6 executable to the directory it will run in
sudo -u pastaaaaaa cp pastaaaaaa /home/pastaaaaaa/

# Run pasta6
sudo -u pastaaaaaa /home/pastaaaaaa/pastaaaaaa
```
