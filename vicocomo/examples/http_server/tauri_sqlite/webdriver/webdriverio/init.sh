#! /bin/sh
sqlite3 -batch tauri.sqlite <<"EOF"
DROP TABLE IF EXISTS counts;
CREATE TABLE counts (val BIGINT);
INSERT INTO COUNTS(val) VALUES (-4711);
DROP TABLE IF EXISTS toughs;
CREATE TABLE toughs
( selec TEXT
, multi TEXT
, radio TEXT
, chbox TEXT
);
INSERT INTO toughs(selec, multi, radio, chbox)
VALUES ('one', 'two three', 'four', '');
EOF
