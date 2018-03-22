<?php

initTest();

$vicocomo = \Vicocomo\Base::instance();

$vicocomo->getDb()->exec(
" DROP TABLE IF EXISTS `Json`;
CREATE TABLE `Json`
( `id`    integer NOT NULL AUTO_INCREMENT
, `dmy`   integer  -- F3 will not save an object wih no values?
, `json`  text
, PRIMARY KEY( `id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8 COLLATE=utf8_swedish_ci
;"
);

class Json {
    use \Vicocomo\SqlModel;

    static function factoryOptions() {
        return [ "json-attrs" => "json"];
    }

    function __construct($factory, $params, $fields, $ttl) {
        $this->initSqlModel($factory, $params, $fields, $ttl);
    }
}
$vicocomo->createSqlModelFactories( [ "\Json"]);

$jsonTest = new \VicocomoTest\SqlModelTest(
        "Json", [ "dmy" => function() { return 1;}]
    );

$jsonTest->testJsonAttr( "json");

