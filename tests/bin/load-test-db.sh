#! /bin/sh

echo "DROP DATABASE vicocomo_test; CREATE DATABASE vicocomo_test;" \
    | mysql -u vicocomo
mysql -u vicocomo vicocomo_test < tests/db/schema.sql
mysql -u vicocomo vicocomo_test < tests/db/test-data.sql

