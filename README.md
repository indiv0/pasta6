# pastaaaaaa

pastaaaaaa (AKA pasta6) is a REST API for uploading arbitrary bytes.
It's a pastebin-alike.

## Quickstart

Copy `.env.example` to `.env`, modify it according to your needs (or leave it to
the **insecure** defaults), then load it:
```sh
cp .env.example .env
vim .env
source .env
```

Replace `172.16.1.92` in `default.conf` to point to your local dev machine's IP:
```
upstream pastaaaaaa {
    server 172.16.1.92:3030;
}
```

Compile the styles with TailwindCSS:
```sh
yarn install
make styles
```

Spin up the nginx and postgres docker containers:
```sh
make dependencies
```

Start the service:
```sh
make watch
```

Dependencies:
* PostgreSQL to store pastes and their metadata

Endpoints:
* GET `/health` always returns 200 OK

## Deploying

Assuming you have a EC2 t2.micro running Ubuntu 20.04 LTS, accessible at
`ubuntu@ec2-xxx-xxx-xxx-xxx.ca-central-1.compute.amazonaws.com`, you can follow
these steps to deploy pasta6 from scratch.

First, modify the EC2 instance's security group to allow tcp/80 and tcp/443 traffic.

Compile & copy the `pasta6` services to the instance:
```sh
make release package deploy
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

Configure the database:
```sh
# Create a user & database for pasta6 to use
sudo -u postgres createuser --interactive # pasta6, y
sudo -u postgres createdb home.p6.rs
sudo -u postgres createdb meta.p6.rs
sudo -u postgres createdb paste.p6.rs
sudo adduser pasta6
sudo -u pasta6 psql postgres # \password, \q
sudo -u postgres psql
# GRANT ALL PRIVILEGES ON DATABASE "home.p6.rs" TO pasta6;
# GRANT ALL PRIVILEGES ON DATABASE "meta.p6.rs" TO pasta6;
# GRANT ALL PRIVILEGES ON DATABASE "paste.p6.rs" TO pasta6;
# \q
```

Extract the services:
```sh
sudo su - pasta6
tar xzvf pasta6.tar.gz
exit
cd /home/pasta6/pasta6
sudo ./install.sh
```

Follow the instructions in the certificates section of this README to generate
production TLS certificates.

Following the `config.example.toml`s throughout this repo, create a unified `config.toml`
for your services to use.

In separate screen instances (e.g. `screen -S home`), launch each service:
```sh
PASTA6_HOST=127.0.0.1 PASTA6_PORT=3030 ./pasta6_home
PASTA6_HOST=127.0.0.1 PASTA6_PORT=3031 ./pasta6_meta
PASTA6_HOST=127.0.0.1 PASTA6_PORT=3032 ./pasta6_paste
```

## Certificates
### Development

To secure the website, generate some certificates with:

```
sudo certbot certonly --manual --preferred-challenges=dns --email=admin@example.com --server https://acme-v02.api.letsencrypt.org/directory --agree-tos -d example.com -d *.example.com
```

Then, generate a strong Diffie-Hellman group:
```
sudo openssl dhparam -out /etc/nginx/dhparam.pem 4096
```

### Production

Install certbot:

    sudo snap install --classic certbot

Install the cloudflare plugin for certbot:

    sudo snap set certbot trust-plugin-with-root=ok
    sudo snap install --beta certbot-dns-cloudflare

Create a file to contain our cloudflare API token:

    sudo mkdir /root/.secrets
    sudo chmod 0700 /root/.secrets/
    sudo touch /root/.secrets/cloudflare.cfg
    sudo chmod 0400 /root/.secrets/cloudflare.cfg

Go to https://dash.cloudflare.com/profile/api-tokens and generate a token with `Zone:DNS:Edit` permissions.
Do not use your global API token!

Edit the `/root/.secrets/cloudflare.cfg` using nano:

    sudo nano /root/.secrets/cloudflare.cfg

Add your cloudflare API token:

    dns_cloudflare_api_token = 0123456789abcdef0123456789abcdef01234567

Generate the certificates:

    sudo /home/pasta6/pasta6/certs-install.sh

You can renew the certificates as well:

    sudo /home/pasta6/pasta6/certs-renew.sh