<?php

initTest();

$vicocomo = \Vicocomo\Base::instance();

function objAttrPath( $name = null) {
    $fileDirNam = \Base::instance()->get( "vicocomoTest.appRoot") . "files";
    if( ! is_dir( $fileDirNam)) {
        mkdir( $fileDirNam);
    }
    if( $name) {
        return "$fileDirNam/$name.txt";
    } else {
        system( "rm -rf $fileDirNam/*");
        return $fileDirNam;
    }
}

function check( $obj, $text, $json, $php, $files, $noFiles = []) {
    $expects = [];
    foreach( $json as $attr => $val) {
        $expects[ "correct $attr JSON"]
            = json_encode( $val) == $obj->mapper->$attr;
    }
    foreach( $php as $attr => $val) {
        $expects[ "correct $attr PHP value"] = $val == $obj->$attr;
    }
    foreach( $files as $fnam) {
        $path = objAttrPath( $fnam);
        $expects[ "file $path"] = file_exists( $path);
    }
    foreach( $noFiles as $fnam) {
        $path = objAttrPath( $fnam);
        $expects[ "no file $path"] = ! file_exists( $path);
    }
    expects( $text, $expects, testMsgFix( $obj));
}

$vicocomo->getDb()->exec(
" DROP TABLE IF EXISTS `Obj`;
CREATE TABLE `Obj`
( `id`   integer NOT NULL AUTO_INCREMENT
, `obj`  text
, `arr`  text
, PRIMARY KEY( `id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8 COLLATE=utf8_swedish_ci
;"
);

class Obj {
    use \Vicocomo\SqlModel;

    static function factoryOptions() {
        return [
                "object-attrs" => [
                        "obj" => "\ObjAttr", "arr" => [ "\ObjAttr", 0]
            ]       ];
    }

    function __construct($factory, $params, $fields, $ttl) {
        $this->initSqlModel($factory, $params, $fields, $ttl);
    }
}

/**
 * A silly object attribute test class.  Each instance represents a
 * filesystem file.  The JSON stored in the database as well as the PHP
 * value is a string, the file name without extension.  When changing the
 * attribute nothing happens until the model object holding the attribute
 * is stored.  Then the old file (if any) is erased and (if name is not
 * NULL) a new text file <name>.txt is created and a random string written
 * to it.
 */
class ObjAttr implements \Vicocomo\ObjectAttr {

    // --- ObjectAttr implementation -------------------------------------

    // we need no constructor, but have to implement it
    function __construct() {
    }

    function fromStore( $decodedData) {
        $this->name = $decodedData;
        $this->path = $this->_makePath( $decodedData);
    }

    function get() {
        return $this->name;
    }

    function set( $name) {
        $this->name = $name;
        return $this;
    }

    function errorsPreventingStore() {
    }

    function toStore() {
        if( $this->_makePath( $this->name) == $this->path) {
            return $this->name;
        }
        $this->delete();
        $this->path = null;
        if( $this->name) {
            $this->path = $this->_makePath( $this->name);
            file_put_contents(
                $this->path, \Vicocomo\Base::instance()->randomHex( 10)
            );
        }
        return $this->name;
    }

    function delete() {
        if( $this->path) {
            unlink( $this->path);
        }
    }

    // --- Private section -----------------------------------------------

    public $name = null;
    public $path = null;

    private function _makePath( $nam = null) {
        if( $nam) {
            return
                \Base::instance()->get( "vicocomoTest.appRoot")
                . "files/$nam.txt";
        }
        return null;
    }
}

$vicocomo->createSqlModelFactories( [ "\Obj"]);

$objTest = new \VicocomoTest\SqlModelTest( "Obj");
$fileDir = objAttrPath();

$obj = $objTest->createObject( [ "obj" => "obj", "arr" => [ "ar0", "ar1"]]);
$obj = factory( "Obj")->modelInstance( $obj->id);
check( $obj, "object created and stored",
    [ "obj" => "obj", "arr" => [ "ar0", "ar1"]],
    [ "obj" => "obj", "arr" => [ "ar0", "ar1"]],
    [ "obj", "ar0", "ar1"]
);

$obj->obj = "ob2";
check( $obj, "new single value set",
    [ "obj" => "obj", "arr" => [ "ar0", "ar1"]],
    [ "obj" => "ob2", "arr" => [ "ar0", "ar1"]],
    [ "obj", "ar0", "ar1"], [ "ob2"]
);
$obj->store();
check( $obj, "new single value stored",
    [ "obj" => "ob2", "arr" => [ "ar0", "ar1"]],
    [ "obj" => "ob2", "arr" => [ "ar0", "ar1"]],
    [ "ob2", "ar0", "ar1"], [ "obj"]
);

$obj->arr = [ "ar0", null, "ar2", "ar3"];
check( $obj, "new array value set",
    [ "obj" => "ob2", "arr" => [ "ar0", "ar1"]],
    [ "obj" => "ob2", "arr" => [ "ar0", "ar2", "ar3"]],
    [ "ob2", "ar0", "ar1"], [ "ar2", "ar3"]
);
$obj->store();
check( $obj, "new array value stored",
    [ "obj" => "ob2", "arr" => [ "ar0", "ar2", "ar3"]],
    [ "obj" => "ob2", "arr" => [ "ar0", "ar2", "ar3"]],
    [ "ob2", "ar0", "ar2", "ar3"], [ "ar1"]
);

$obj->arr = [ null, "ar4"];
check( $obj, "zombie created",
    [ "obj" => "ob2", "arr" => [ "ar0", "ar2", "ar3"]],
    [ "obj" => "ob2", "arr" => [ "ar4", null, null]],
    [ "ob2", "ar0", "ar2", "ar3"], [ "ar4"]
);
$obj->store();
check( $obj, "zombie not stored",
    [ "obj" => "ob2", "arr" => [ "ar4"]],
    [ "obj" => "ob2", "arr" => [ "ar4"]],
    [ "ob2", "ar4"], [ "ar0", "ar2", "ar3"]
);

