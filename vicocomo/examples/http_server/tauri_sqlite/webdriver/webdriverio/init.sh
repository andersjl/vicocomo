#! /bin/sh
sqlite3 -batch tauri.sqlite <<"EOF"
DROP TABLE IF EXISTS counts;
CREATE TABLE counts (val BIGINT);
INSERT INTO COUNTS(val) VALUES (-4711);
EOF
