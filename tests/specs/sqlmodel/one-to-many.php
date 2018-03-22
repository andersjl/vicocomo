<?php

initTest();

$vicocomo = \Vicocomo\Base::instance();

$vicocomo->getDb()->exec(
" DROP TABLE IF EXISTS `One`;
CREATE TABLE `One`
( `id`    integer NOT NULL AUTO_INCREMENT
, `data`  integer
, PRIMARY KEY( `id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8 COLLATE=utf8_swedish_ci
; DROP TABLE IF EXISTS `Thru`
; CREATE TABLE `Thru`
( `id`         integer NOT NULL AUTO_INCREMENT
, `fkOne`      integer
, `fkOneToo`   integer
, `oneId`      integer
, `oneTooId`   integer
, `fkMany`     integer
, `fkManyToo`  integer
, `manyId`     integer
, `manyTooId`  integer
, `data`       integer
, PRIMARY KEY( `id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8 COLLATE=utf8_swedish_ci
; DROP TABLE IF EXISTS `Many`
; CREATE TABLE `Many`
( `id`        integer NOT NULL AUTO_INCREMENT
, `fkOne`     integer
, `fkOneToo`  integer
, `oneId`     integer
, `oneTooId`  integer
, `data`      integer
, PRIMARY KEY( `id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8 COLLATE=utf8_swedish_ci
; "
);

global $oneOpts;
$oneOpts = [];
global $manyOpts;
$manyOpts = [];
class One {
    use \Vicocomo\SqlModel;

    static function factoryOptions() {
        global $oneOpts;
        return array_merge( [ "compare" => "data"], $oneOpts);
    }

    function __construct( $factory, $params, $fields, $ttl) {
        $this->initSqlModel( $factory, $params, $fields, $ttl);
    }
}
class OneToo {
    use \Vicocomo\SqlModel;

    static function factoryOptions() {
        global $oneOpts;
        return array_merge(
                [ "table-name" => "One", "compare" => "data"], $oneOpts
            );
    }

    function __construct( $factory, $params, $fields, $ttl) {
        $this->initSqlModel( $factory, $params, $fields, $ttl);
    }
}
class Thru {
    use \Vicocomo\SqlModel;

    static function factoryOptions() {
        return [ "compare" => "data"];
    }

    function __construct( $factory, $params, $fields, $ttl) {
        $this->initSqlModel( $factory, $params, $fields, $ttl);
    }
}
$vicocomo->createSqlModelFactories( "\Thru");
class Many {
    use \Vicocomo\SqlModel;

    static function factoryOptions() {
        global $manyOpts;
        return array_merge( [ "compare" => "data"], $manyOpts);
    }

    function __construct( $factory, $params, $fields, $ttl) {
        $this->initSqlModel( $factory, $params, $fields, $ttl);
    }
}
class ManyToo {
    use \Vicocomo\SqlModel;

    static function factoryOptions() {
        global $manyOpts;
        return array_merge(
                [ "table-name" => "Many", "compare" => "data"], $manyOpts
            );
    }

    function __construct( $factory, $params, $fields, $ttl) {
        $this->initSqlModel( $factory, $params, $fields, $ttl);
    }
}

$thruTest = new \VicocomoTest\SqlModelTest(
        "Thru", [ "data" => randomUniqueIntegerFactory( -999, 999)]
    );
foreach ( [ "One", "OneToo"] as $oneMdl) {
    foreach ( [ "Many", "ManyToo"] as $manyMdl) {
        foreach ( [ null, lcfirst( $oneMdl) . "Id"] as $forKey) {
            foreach ( [ null, "restrict", "set-null", "cascade", "Thru"
                ] as $onDelOrThru
            ) {
                foreach(
                    "Thru" == $onDelOrThru
                    ? [ null, lcfirst( $manyMdl) . "Id"] : [ null]
                    as $remKey
                ) {
                    message( "$oneMdl to $manyMdl"
                        . ( $onDelOrThru
                            ? ( "Thru" == $onDelOrThru
                                ? " (through Thru)" : " ($onDelOrThru)"
                            ) : ""
                        ) . ( $forKey ? ", foreign key: $forKey" : "")
                        . ( $remKey ? ", remote key: $remKey" : "")
                    );
//testLog( "------------", $oneMdl, $manyMdl, $forKey, $onDelOrThru, $remKey);
                    clearDb();
                    $hasManyOpts
                        = [ "remote-name" => $manyMdl, "on-delete" => null];
                    $oneTestOpts   = [];
                    $belongsToOpts = [ "remote-name" => $oneMdl];
                    if( $forKey) {
                        $hasManyOpts[ "foreign-key"]   = $forKey;
                        $belongsToOpts[ "foreign-key"] = $forKey;
                        $oneTestOpts[ "foreign-key"]   = $forKey;
                    }
                    if( "Thru" == $onDelOrThru) {
                        $hasManyOpts[ "through"]      = "Thru";
                        $oneTestOpts[ "join-test"]    = $thruTest;
                        $oneTestOpts[ "ordered-join"] = orderedAttr(
                            "data"
                        );
                        if( $remKey) {
                            $hasManyOpts[ "remote-key"] = $remKey;
                            $oneTestOpts[ "remote-key"] = $remKey;
                        }
                    } else {
                        $hasManyOpts[ "on-delete"] = $onDelOrThru;
                    }
                    $oneOpts = [ "has-many" => [ $hasManyOpts]];
                    $vicocomo->createSqlModelFactories( "\\$oneMdl");
                    $manyOpts = [ "belongs-to" => [ $belongsToOpts]];
                    $vicocomo->createSqlModelFactories( "\\$manyMdl");
                    $oneTest = new \VicocomoTest\SqlModelTest(
                        $oneMdl,
                        [ "data" => randomUniqueIntegerFactory( -999, 999)]
                    );
                    $manyTest = new \VicocomoTest\SqlModelTest(
                        $manyMdl,
                        [ "data" => randomUniqueIntegerFactory( -999, 999)]
                    );
                    $oneTestOpts[ "remote-test"] = $manyTest;
                    $oneTestOpts[ "on-delete"]   = $hasManyOpts[ "on-delete"];
                    $oneTestOpts[ "ordered-remote"] = orderedAttr( "data");
                    $oneTest->testHasMany( [ $manyMdl => $oneTestOpts]);
                    if( "Thru" != $onDelOrThru) {
                        $manyTest->testBelongsTo(
                            [ lcfirst( $oneMdl) => [ $oneTest]]
                        );
                    }
                }
            }
        }
    }
}

