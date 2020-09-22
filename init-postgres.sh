#!/bin/sh
set -e

function create_user_and_database() {
    local database=$1
    echo "Creating user and database '$database'"
    psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" <<-EOSQL
        CREATE USER "$database" WITH PASSWORD '$database';
        CREATE DATABASE "$database";
        GRANT ALL PRIVILEGES ON DATABASE "$database" TO "$database";
EOSQL
}

if [ -n "$POSTGRES_DBS" ]; then
    echo "Created of multiple databases required: $POSTGRES_DBS"
    for db in $(echo $POSTGRES_DBS | tr ',' ' '); do
        create_user_and_database $db
    done
    echo "Created postgres databases"
fi