#!/bin/bash

set -e
set -u

function create_user() {
	local user=$1
	echo "  Creating user '${user}_login' "
	psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" <<-EOSQL
	    CREATE USER "${user}_login" WITH PASSWORD 'login';
	    GRANT ALL PRIVILEGES ON DATABASE $user TO "${user}_login";
        GRANT ALL PRIVILEGES ON SCHEMA public TO "${user}_login";
EOSQL
}

if [ -n "$POSTGRES_MULTIPLE_USERS" ]; then
	echo "Multiple users creation requested: $POSTGRES_MULTIPLE_USERS"
	for user in $(echo $POSTGRES_MULTIPLE_USERS | tr ',' ' '); do
		create_user $user
	done
	echo "Multiple users created"
fi
