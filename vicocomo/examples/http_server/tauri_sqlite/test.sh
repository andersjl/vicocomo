#! /bin/sh

cd src-tauri

sqlite3 -batch tauri.sqlite <<"EOF"
DROP TABLE IF EXISTS counts;
CREATE TABLE counts (val BIGINT);
INSERT INTO counts(val) VALUES (-4711);
EOF

./target/release/vicocomo-example-http-server-tauri-sqlite
