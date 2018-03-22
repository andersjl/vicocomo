<?php

namespace VicocomoTest;

/*
 * Methods and properties for testing.
*/
class Utils extends \Prefab {
    use \Vicocomo\GeneralUtils;
    use RandomValues;
    use ValueProducers;

    const NO_EXPECTATIONS = "905c951b712d6c2ed9955079c467790267eef9";

    static function initTest() {
        error_reporting( E_ALL);
        $utils = self::instance();
        $utils->clearDb();
        return $utils;
    }

    public $enableQuiet = true;
    public $f3;
    public $vicocomo;

    function __construct() {
        $this->vicocomo = \Vicocomo\Base::instance();
        $this->f3       = $this->vicocomo->f3;
    }

    /*
     * Clear all tables, currently only in a MySql database.
     */
    function clearDb() {
        $db = $this->vicocomo->getDb();
        $driver = $db->driver();
        if( "mysql" != $driver) {
            throw new \Exception( "cannot clear DB driver $driver");
        }
        $deletes = "";
        foreach( array_map(
            function( $info) {
                return $info["TABLE_NAME"];
            },
            $db->exec(
                "SELECT TABLE_NAME FROM INFORMATION_SCHEMA.TABLES" .
                " WHERE TABLE_TYPE   = 'BASE TABLE'" .
                "   AND TABLE_SCHEMA = '" . $db->name() . "';"
            )
        ) as $table) {
            $deletes .= " DELETE FROM `$table`;";
        }
        $db->exec( $deletes);
    }

    /**
     * Mock request with suppressed output and return the response.
     *
     * $request, $params":  See \Base::instance()->mock().
     *
     * $responseCode is filled with the return value of
     * http_response_code( 200), i.e. the test program will return OK status
     * in the response regardless.
     */
    function mock( $request, $params = null, &$responseCode = null) {
        $wasQuiet = $this->f3->get( "QUIET");
        if( $this->enableQuiet && ! $wasQuiet) {
            $this->f3->set( "QUIET", true);
        }
        $this->f3->mock( $request, $params);
        if( ! $wasQuiet) {
            $this->f3->set( "QUIET", false);
        }
        $responseCode = http_response_code( 200);
        return $this->f3->get( "RESPONSE");
    }

    /*
     * Execute a function that may cause an error and capture the error.
     *
     * $func is the possibly ill-behaved function.  It may take one parameter,
     * the Utils instance.
     *
     * Returns the arguments to the error handler or null if no error.
     */
    function captureError($func) {
        $capturedError = null;
        $oldHandler = set_error_handler(
                function () use (&$capturedError) {
                    $capturedError = func_get_args();
                    return true;
                }
            );
        call_user_func($func, $this);
        set_error_handler($oldHandler);
        return $capturedError;
    }

    /*
     * The array of all the results collected by the storeResult method.
     */
    function results() {
        return $this->_results;
    }

    /*
     * True if all tests stored in results() have passed.
     */
    function passed() {
        foreach( $this->_results as $result) {
            if( $result["status"] === false) {
                return false;
            }
        }
        return true;
    }

    /*
     * The array of all the $extra-s collected by the storeResult method with
     * NO_EXPECTATIONS.
     */
    function notes() {
        $result = [];
        foreach( $this->_results as $stored) {
            $note = $stored["note"];
            if( $note) {
                $result[] = $note;
            }
        }
        return $result;
    }

    /*
     * Store an entry in the array that is accessed by the results method.
     *
     * The entry is an associative array with keys "status", "text", "line",
     * and "note".
     *
     * $status indicates success or failure.  The magic status
     * Orjhlab\Test::NO_EXPECTATIONS is stored as NULL, all other values are
     * cast to boolean.
     *
     * $text is a string that describes the test.
     *
     * $extra is an extra string that is appended to $text if not $status,
     * except if $status is Orjhlab\Test::NO_EXPECTATIONS, in which case it is
     * stored in the key "note".
     *
     * If the stored status is false, the line no from the test file is
     * stored.
     *
     * Returns the stored array.
     */
    function storeResult( $status, $text, $extra = null) {
        if( $extra && !$status) {
            $text .= " (" . $extra . ")";
        }
        $note = $extra && $status === self::NO_EXPECTATIONS ? $extra : null;
        $result           = [];
        $result["status"] = null;
        $result["text"]   = str_repeat( "  ", $this->_indent) . $text;
        $result["line"]   = null;
        $result["note"]   = $note;
        if( $status !== self::NO_EXPECTATIONS) {
            $result["status"] = (bool)$status;
        }
        foreach( debug_backtrace() as $frame) {
            if( isset( $frame["file"])
                && strpos( $frame["file"], $this->_tree) === 0
            ) {
                $result["line"] = $frame["line"];
                break;
            }
        }
        return $this->_results[] = $result;
    }

    /*
     * Call the storeResult() method, see that.
     *
     * If $onlyOnError is trueish and not a string, storeResult() is not
     * called if $test is trueish.
     *
     * If $onlyOnError is a string, it is the $extra arg to storeResult().
     *
     * Returns the stored boolean result or null if none stored.
     */
    function expect( $test, $text, $onlyOnError = false) {
        $storeSuccess = !$onlyOnError || is_string( $onlyOnError);
        $extraErrorText = is_string( $onlyOnError) ? $onlyOnError : null;
        if( !$test || $storeSuccess) {
            $result = $this->storeResult( $test, $text, $extraErrorText
                )["status"];
        } else {
            $result = null;
        }
        return $result;
    }

    /*
     * Run expect for each item in $tests as $text => $test and quiet.
     *
     * Run expect an extra time with all tests and-ed, $summary, $onlyOnError.
     */
    function expects( $summary, $tests, $onlyOnError = false) {
        $success = true;
        foreach( $tests as $text => $test) {
            $success = $success && $test;
            $this->expect( $test, "  " . $text, true);
        }
        return $this->expect( $success, $summary, $onlyOnError);
    }

    /*
     * Put $msg in the test result array.
     */
    function message( $msg) {
        $this->storeResult(self::NO_EXPECTATIONS, "--- $msg");
    }

    function runTest( $path) {
        set_time_limit( 600);
        $this->_results = [];
        $this->_indent = 0;
        if( is_dir( $path)) {
            if(substr( $path, -1) !== "/") {
                $path .= "/";
            }
        } elseif(substr( $path, -1) === "/") {
            $path = rtrim( $path, "/");
        }
        $this->_doTest( $path);
        return [$this->_results, $this->passed(), $this->notes() ];
    }

    /**
     * Utility to get the value of an object property that may be protected or
     * private.
     */
    function getPrivate( $obj, $name) {
        $attrs = (array)$obj;
        foreach( $attrs as $key => $val) {
            $aux = explode( "\0", $key);
            if( $aux[count( $aux) - 1] === $name) {
                return $val;
            }
        }
        return null;
    }

    /**
     * Utility to handle flotaing number equality.
     */
    function equal( $a, $b) {
        if( !is_float( $a) && !is_float( $b)) {
            return $a == $b;
        }
        if( !is_float( $a)) {
            $a = (float)$a;
        }
        if( !is_float( $b)) {
            $b = (float)$b;
        }
        return abs( $a - $b) / (abs( $a) + abs( $b)) < 1e-9;
    }

    /**
     * (array) obj, but with the protected and private keys reduced to just
     * the property name.
     */
    function castObject( $obj) {
        $attrs = (array)$obj;
        $result = [];
        foreach( $attrs as $key => $val) {
            $aux = explode( "\0", $key);
            $result[$aux[count( $aux) - 1]] = $val;
        }
        return $result;
    }

    function testMsgFixDo( $arr, $html, $depth) {
        if( $depth > 3) {
            return "array with keys " . implode( ", ", array_keys( $arr));
        }
        $result = [];
        foreach( $arr as $key => $elem) {
            if( is_object( $elem)) {
                if( method_exists( $elem, "cast")) {
                    $sub = $elem->cast();
                } else {
                    $sub = $this->castObject( $elem);
                }
            } elseif( is_array( $elem)) {
                $sub = $elem;
            } else {
                $sub = null;
                if( $html && is_string( $elem)) {
                    $elem = htmlspecialchars( $elem);
                }
            }
            $result[$key] = $sub
                ? $this->testMsgFixDo( $sub, $html, $depth + 1) : $elem;
        }
        return $result;
    }

    function testMsgFix( $msg, $html = false) {
        if( !is_array( $msg)) {
            $msg = [$msg];
        }
        return var_export( $this->testMsgFixDo( $msg, $html, 0), true);
    }

    function testLog() {
        error_log(
            date( "c") . " " . $this->testMsgFix( func_get_args()) . PHP_EOL,
            3,
            "../log.txt"
        );
    }

    /**
     * Insert debug information in the test output.
     */
    function testExpect() {
        $this->expect( false, $this->testMsgFix( func_get_args()));
    }

    /**
     * Use as callback to check sorting on attribute $attr.
     */
    function orderedAttr( $attr) {
        return function( $o1, $o2) use( $attr) {
                $a1 = is_array( $o1) ? $o1[ $attr] : $o1->$attr;
                $a2 = is_array( $o2) ? $o2[ $attr] : $o2->$attr;
                return ( $a1 === 0 || $a1) && ( $a2 === 0 || $a2)
                        && $a1 <= $a2;
            };
    }

    private $_results;
    private $_passed;
    private $_notes;
    private $_indent;
    private $_tree;

    private function _doTest( $spec) {
        if( $spec != $this->_tree) {
            $this->storeResult(
                self::NO_EXPECTATIONS, str_replace( $this->_tree, '', $spec)
            );
            $this->_indent += 1;
        }
        if( is_dir( $spec)) {
            $spec = $this->ensureSlash( $spec);
            foreach( glob( $spec . "*") as $nextSpec) {
                $this->_doTest( $nextSpec);
            }
        } elseif( preg_match( '/.*\.php$/i', $spec)) {
            require $spec;
        } else {
            $this->storeResult(
                self::NO_EXPECTATIONS, "not handled: " . $spec
            );
        }
        $this->_indent -= 1;
    }
}

