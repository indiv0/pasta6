#!/bin/sh
set -e
# nginx
[ -f /etc/nginx/dhparam.pem ] || openssl dhparam -out /etc/nginx/dhparam.pem 4096
sed -i 's/172.16.1.92/127.0.0.1/g' nginx/default.conf
sed -i 's/uh.rs/p6.rs/g' nginx/default.conf
sed -i 's/\/usr\/share\/nginx\/html/\/srv\/www\/p6.rs/g' nginx/default.conf
sed -i 's/uh.rs/p6.rs/g' nginx/certs.conf
cp nginx/certs.conf /etc/nginx/snippets/certs.conf
cp nginx/options-ssl-nginx.conf /etc/nginx/snippets/options-ssl-nginx.conf
cp nginx/default.conf /etc/nginx/sites-available/p6.rs
ln -sf /etc/nginx/sites-available/p6.rs /etc/nginx/sites-enabled/p6.rs
sudo rm -f /etc/nginx/sites-enabled/default
# static files
mkdir -p /srv/www/p6.rs
cp static/styles.css static/robots.txt /srv/www/p6.rs
# executables
mkdir -p /home/pasta6/bin
cp pasta6_home pasta6_meta pasta6_paste /home/pasta6/bin
# restart nginx
nginx -t
nginx -s reload