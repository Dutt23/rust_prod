#!/usr/bin/env bash
set -x
set -eo pipefail

if ! [ -x "$(command -v psql)" ]; then
  echo >&2 "Error: psql is not installed."
  exit 1
fi
if ! [ -x "$(command -v sqlx)" ]; then
  echo >&2 "Error: sqlx is not installed."
  echo >&2 "Use:"
  echo >&2 "    cargo install --version=0.5.7 sqlx-cli --no-default-features --features postgres"
  echo >&2 "to install it."
  exit 1
fi

# Check if a custom user has been set, otherwise default to 'postgres'
DB_USER=${POSTGRES_USER:=root}
# Check if a custom password has been set, otherwise default to 'password'
DB_PASSWORD="${POSTGRES_PASSWORD:=secret}"
# Check if a custom database name has been set, otherwise default to 'newsletter'
DB_NAME="${POSTGRES_DB:=news_letter}"
# Check if a custom port has been set, otherwise default to '5432'
DB_PORT="${POSTGRES_PORT:=5430}"
# Launch postgres using Docker

# Allow to skip Docker if a dockerized Postgres database is already running
# if [[ -z "${SKIP_DOCKER}" ]]
# then
#   docker run \
#       -e POSTGRES_USER=${DB_USER} \
#       -e POSTGRES_PASSWORD=${DB_PASSWORD} \
#       -e POSTGRES_DB=${DB_NAME} \
#       -p "${DB_PORT}":5432 \
#       -d postgres \
#       postgres -N 1000
# fi
# docker run \
#   -e POSTGRES_USER=${DB_USER} \
#   -e POSTGRES_DB=${DB_NAME} \
#   -p "${DB_PORT}":5432 \
#   -d postgres \
#   postgres -N 1000
  # ^ Increased maximum number of connections for testing purposes

# until psql -h "localhost" -U "${DB_USER}" -p "${DB_PORT}" -d "postgres" -c '\q'; do
#   >&2 echo "Postgres is still unavailable - sleeping"
# sleep 1 done
# >&2 echo "Postgres is up and running on port ${DB_PORT} - running migrations now!"
# export DATABASE_URL=postgres://${DB_USER}:${DB_PASSWORD}@localhost:${DB_PORT}/${DB_NAME}
# sqlx database create
# sqlx migrate run
# >&2 echo "Postgres has been migrated, ready to go!"
export DATABASE_URL=postgres://${DB_USER}:${DB_PASSWORD}@localhost:${DB_PORT}/${DB_NAME}
sqlx database create
sqlx migrate run

# For running postgres docker locally
# docker pull postgres && docker run --name postgres -p 5430:5432 -e POSTGRES_USER=root -e POSTGRES_PASSWORD=secret -d postgres
# docker exec -it postgres psql --username=root news_letter