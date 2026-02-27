#!/bin/bash
set -e

# Create multiple databases for BROZ microservices
# This script is run by the PostgreSQL Docker entrypoint

DATABASES="broz_auth broz_user broz_matching broz_messaging broz_notification broz_moderation broz_analytics"

for db in $DATABASES; do
    echo "Creating database: $db"
    psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname "$POSTGRES_DB" <<-EOSQL
        SELECT 'CREATE DATABASE $db' WHERE NOT EXISTS (SELECT FROM pg_database WHERE datname = '$db')\gexec
        GRANT ALL PRIVILEGES ON DATABASE $db TO $POSTGRES_USER;
EOSQL
done

echo "All databases created successfully"
