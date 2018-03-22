<?php

initTest();

$vicocomo = \Vicocomo\Base::instance();

$vicocomo->getDb()->exec( "
DROP TABLE IF EXISTS `SimpleDbRequired`;
CREATE TABLE `SimpleDbRequired`
( `id`              integer NOT NULL AUTO_INCREMENT
, `requiredFieldA`  integer NOT NULL
, `requiredFieldB`  integer NOT NULL
, PRIMARY KEY( `id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8 COLLATE=utf8_swedish_ci;
SHOW WARNINGS;
DROP TABLE IF EXISTS `SimpleSwRequired`;
CREATE TABLE `SimpleSwRequired`
( `id`              integer NOT NULL AUTO_INCREMENT
, `requiredFieldA`  integer
, `requiredFieldB`  integer
, `disallowedField` integer
, `optionalField`   integer
, `defaultDbField`  integer DEFAULT 42
, `defaultSwField`  integer
, `uniqueFieldA`    integer
, `uniqueFieldB1`   integer
, `uniqueFieldB2`   integer
, `sortedFieldA`    integer
, `sortedFieldB`    integer
, PRIMARY KEY( `id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8 COLLATE=utf8_swedish_ci;
SHOW WARNINGS;
DROP TABLE IF EXISTS `SimpleCompareFunc`;
CREATE TABLE `SimpleCompareFunc`
( `id`  integer NOT NULL AUTO_INCREMENT
, `a`   integer
, `b`   integer
, PRIMARY KEY( `id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8 COLLATE=utf8_swedish_ci;
SHOW WARNINGS;"
);

class SimpleDbRequired {
    use \Vicocomo\SqlModel;

    static function factoryOptions() {
    }

    function __construct($factory, $params, $fields, $ttl) {
        $this->initSqlModel($factory, $params, $fields, $ttl);
    }
}

class SimpleSwRequired {
    use \Vicocomo\SqlModel;

    static function factoryOptions() {
        return ["compare" => ["sortedFieldA", "sortedFieldB"]];
    }

    function __construct($factory, $params, $fields, $ttl) {
        $this->initSqlModel($factory, $params, $fields, $ttl);
    }

    protected function errorsPreventingStore() {
        $result = [];
        $this->reportFalsy($result, ["requiredFieldA", "requiredFieldB"]);
        if ($this->disallowedField) {
            $result[] = "not allowed";
        }
        if (!$this->defaultSwField) {
            $this->defaultSwField = 17;
        }
        $this->reportDuplicate($result, ["uniqueFieldA"]);
        $this->reportDuplicate(
            $result, ["uniqueFieldB1", "uniqueFieldB2"]
        );
        return $result ? : false;
    }
}

class SimpleCompareDesc {
    use \Vicocomo\SqlModel;

    static function factoryOptions() {
        return [
                "table-name" => "SimpleSwRequired",
                "compare"    => ["sortedFieldA DESC", "sortedFieldB DESC"]
            ];
    }

    function __construct($factory, $params, $fields, $ttl) {
        $this->initSqlModel($factory, $params, $fields, $ttl);
    }
}

global $descending;
class SimpleCompareFunc {
    use \Vicocomo\SqlModel;

    static function factoryOptions()  {
        return [ "compare"
                => function($o1, $o2) {
                        global $descending;
                        if( $descending) {
                            return $o1->a * $o1->b - $o2->a * $o2->b;
                        } else {
                            return $o2->a * $o2->b - $o1->a * $o1->b;
                        }
                    }
            ];
    }

    function __construct($factory, $params, $fields, $ttl) {
        $this->initSqlModel($factory, $params, $fields, $ttl);
    }
}

$vicocomo->createSqlModelFactories( [
        "\SimpleDbRequired", "\SimpleSwRequired", "\SimpleCompareDesc",
        "\SimpleCompareFunc"
    ]
);

message("allowing errors when missing required");
$simpleTest = new \VicocomoTest\SqlModelTest(
    "SimpleDbRequired", [
        "requiredFieldA" => orderedIntegerFactory(100),
        "requiredFieldB" => orderedIntegerFactory(200),
    ]
);
$simpleTest->testRequired(["capture-errors" => true]);
message("no errors accepted when missing required");
$simpleTest = new \VicocomoTest\SqlModelTest(
    "SimpleSwRequired", [
        "requiredFieldA" => orderedIntegerFactory(100),
        "requiredFieldB" => orderedIntegerFactory(200),
    ]
);
$simpleTest->testRequired();
$simpleTest->testDisallowed(["disallowedField" => 4]);
$simpleTest->testPersistence( [
        "requiredFieldA" => 100,
        "requiredFieldB" => 200,
        "optionalField"  => 300
    ]
);
$simpleTest->testDefault(["defaultDbField" => 42, "defaultSwField" => 17]);
$simpleTest->testUnique( [
        "uniqueFieldA" => [42, 43], [
            "uniqueFieldB1" => [1, 2], "uniqueFieldB2" => 3
        ]
    ]
);
$simpleTest->testSorted( [
        "sortedFieldA" => "int",
        "sortedFieldB"
        => function() {
                return rand( -1, 1);
            },
    ]
);
$simpleTest = new \VicocomoTest\SqlModelTest(
    "SimpleCompareDesc", ["requiredFieldA" => orderedIntegerFactory(100)]
);
$simpleTest->testSorted( [
        "sortedFieldA" => "int",
        "sortedFieldB" => randomUniqueIntegerFactory( -17, 17),
    ], ["desc" => true]
);
$simpleTest = new \VicocomoTest\SqlModelTest("SimpleCompareFunc");
$descending = false;
$simpleTest->testSorted(
    function( $o1, $o2) {
        return $o1->a * $o1->b <= $o2->a * $o2->b;
    }, ["text" => "product of a and b"]
);
$descending = true;
$simpleTest->testSorted(
    function( $o1, $o2) {
        return $o1->a * $o1->b >= $o2->a * $o2->b;
    }, ["text" => "product of a and b descending"]
);

