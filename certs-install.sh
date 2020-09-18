#!/bin/sh
set -e
snap install --classic certbot
snap set certbot trust-plugin-with-root=ok
snap install --beta certbot-dns-cloudflare
certbot certonly \
    --server https://acme-v02.api.letsencrypt.org/directory \
    --email admin@p6.rs \
    --agree-tos \
    --dns-cloudflare \
    --dns-cloudflare-credentials /root/.secrets/cloudflare.cfg \
    -d p6.rs -d *.p6.rs