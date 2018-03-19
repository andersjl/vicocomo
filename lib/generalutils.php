<?php

namespace Vicocomo;

/**
 * A bunch of nice-to-have general utilities that do not depend on the state
 * of the object using them, except...
 *
 * ... some functions use the property $this->f3, but this is not declared in
 * the trait, so a class using the trait and those functions has to set such a
 * property to \Base::instance().
 */
trait GeneralUtils {

// --- PHP utilities ---------------------------------------------------------

    /** is_callable() but not a string */
    function callable( $value) {
        return ! is_string( $value) && is_callable( $value);
    }

    /**
     * Utility to get the value of an object property that may be protected or
     * private.
     */
    function getPrivate( $obj, $name) {
        $attrs = (array) $obj;
        foreach( $attrs as $key => $val) {
            $aux = explode( "\0", $key);
            if( $aux[ count( $aux) - 1] === $name) {
                return $val;
            }
        }
        return null;
    }

// --- Class name utilities --------------------------------------------------

    /**
     * Return the classname of $obj with any namespace stripped.  $obj may be
     * an instance or the result of get_class() or a class name without
     * namespace.
     */
    function shortClassName( $obj) {
        $name = is_object( $obj) ? get_class( $obj) : $obj;
        $bs = strrpos( $name, "\\");
        if( false === $bs) {
            return $name;
        }
        return substr( $name, $bs + 1);
    }

    /**
     * Compute the full class name including namespace from $name.
     *
     * $name is a short or long classname, with or without "\" as the first
     * character.
     *
     * $defaultNamespace is a default namespace if $name contains no "\".
     *
     * Ensures that the returned full class name always starts with "\".
     */
    function fullClassName( $name, $defaultNamespace = "\\") {
        $bs = strpos( $name, "\\");
        if( 0 === $bs) {
            return $name;
        }
        if( false === $bs) {
            if( 0 !== strpos( $defaultNamespace, "\\")) {
                $defaultNamespace = "\\" . $defaultNamespace;
            }
            if( "\\" !== substr( $defaultNamespace, -1)) {
                $defaultNamespace .= "\\";
            }
            return $defaultNamespace . $name;
        }
        return "\\" . $name;
    }

    /**
     * Compute the namespace from $name.
     *
     * $name is a short or long classname, with or without "\" as the first
     * character, or an empty string.
     *
     * $default is a default namespace if $name is falsy or contains no "\".
     *
     * Ensures that the returned namespace always starts and ends with "\".
     */
    function getNamespace( $name, $default = "\\") {
        $bs = strrpos( $name, "\\");
        $ns = false === $bs
              ? ( "\\" === substr( $default, -1) ? $default : "$default\\")
              : substr( $name, 0, $bs + 1);
        return $ns[ 0] === "\\" ? $ns : "\\$ns";
    }

// --- String utilities ------------------------------------------------------

    /**
     * Remove all whitespace and make lowercase.
     */
    function normalizeString( $s) {
        return preg_replace( "/\s+/", "", mb_strtolower( $s, "UTF-8"));
    }

    /**
     * levenshtein() ignoring case and whitespace
     *
     * $template is supposed to be normalizeString()-ed, $string will be.
     */
    function levenshtein( $template, $string) {
        return levenshtein( $template, $this->normalizeString( $string));
    }

// --- Array utilities -------------------------------------------------------

    /**
     * Ensure that a value is an array.
     *
     * Returns $val if it is an array, [ $val] otherwise.
     */
    function ensureArray( $val) {
        return is_array( $val) ? $val : [ $val];
    }

    /**
     * Ensure that one ore more keys are set in an array.
     *
     * $keys is the key or an array of them.  A default value can be supplied
     * by making the key an array [ [ <key>, <default value>]].
     *
     * $arr is the array or NULL.
     *
     * Returns an array where the value of each key in $keys is defined.  If
     * it was not set on entry the value will be the given default or NULL.
     */
    function ensureKey( $keys, $arr) {
        if( NULL === $arr) {
            $arr = [];
        }
        $keys = is_array( $keys) ? $keys : [ $keys];
        foreach( $keys as $keyOrArr) {
            if( is_array( $keyOrArr)) {
                $def = $keyOrArr;
                $key = array_shift( $def);
                $default = array_shift( $def);
            } else {
                $key     = $keyOrArr;
                $default = null;
            }
            if( ! isset( $arr[ $key])) {
                $arr[ $key] = $default;
        }   }
        return $arr;
    }

    /**
     * Create an associative array from the values of another array.
     *
     * $func is a callable that takes one arg, a value from $positional, and
     * returns a key-value pair as a two-element array.
     *
     * $valueArr is the array with the values to use.  Its keys are ignored.
     *
     * Returns the associative array built from $func's return values.
     */
    function buildAssociativeArray( $func, $valueArr) {
        $result = [];
        foreach( $valueArr as $value) {
            $newKeyValue = call_user_func( $func, $value);
            $result[ $newKeyValue[ 0]] = $newKeyValue[ 1];
        }
        return $result;
    }

    /**
     * Get a value from an associative array, return $default if not set.
     */
    function getArrayVal( $key, $arr, $deflt = null) {
        return is_array( $arr) && isset( $arr[ $key]) ? $arr[ $key] : $deflt;
    }

    /**
     * Find the key of the first value in $arr for which $cond returns truthy
     * or FALSE if none found.
     */
    function findFirst( $arr, $cond) {
        foreach( $arr as $key => $value) {
            if( call_user_func( $cond, $value)) {
                return $key;
            }
            return false;
        }
    }

    function powerSet() {
        $arr = func_get_args();
        if( 1 == count( $arr) && is_array( $arr[ 0])) {
            $arr = $arr[ 0];
        }
        if( 0 == count( $arr)) {
            return[ []];
        }
        $last = array_pop( $arr);
        $rest = $this->powerSet( $arr);
        return array_merge(
                $rest,
                array_map(
                    function( $x) use( $last) {
                        return array_merge( $x,[ $last]);
                    }, $rest
            )   );
    }

// --- Date and time utilities -----------------------------------------------

    /**
     * Return next midnight as a Unix timestamp.
     */
    function nextMidnight( $timestamp) {
        return strtotime( date( "Y-m-d", $timestamp + 86400));
    }

// --- File path utilities ---------------------------------------------------

    /**
     * Ensure that $dir ends in a slash.
     */
    function ensureSlash( $dir) {
        return "/" == substr( $dir, -1) ? $dir : $dir . "/";
    }

// --- HTML path utilities ---------------------------------------------------

    /**
     * Return the public HTML root made to absolute path with trailing slash.
     */
    function publicDir() {
        return realpath( ".") . "/";
    }

    /**
     * Compute a link from the parameters.
     *
     * $relPath is the path relative to the HTML root, with no slash at the
     * beginning.
     *
     * If $mtime is truthy the file's mtime is inserted before the extension
     * to bust the browser cahce on change.  Requires rewriting the URL, e.g.
     * for Apache, supposing you use makeHref() only for .js and .css files:
     *   RewriteRule ^(.*)\.[\d]{10}\.(css|js)$ $1.$2 [L]
     *
     * If the disk file exists, returns $relPath, possibly rewritten as above.
     * If not, returns FALSE.
     */
    function makeHref( $relPath, $mtime = true) {
        $path = $this->publicDir() . $relPath;
        if( ! file_exists( $path)) {
            return false;
        }
        return
            $this->f3->get( "BASE") . "/"
            . ( $mtime
                ? preg_replace(
                        '{\\.([^./]+)$}', "." . filemtime( $path) . ".$1",
                        $relPath
                    )
                : $relPath
            );
    }

// --- F3 hive utilities -----------------------------------------------------

    /**
     * \Base::instance()->exists() with a default if not exists.
     */
    function getF3( $key, $default = null) {
        if( $this->f3->exists( $key, $val)) {
            return $val;
        } else {
            return $default;
        }
    }

    /**
     * \Base::instance()->push(), but ensure that $val is only pushed once.
     *
     * If $strict, comparision is by ===, default ==.
     *
     * Returns $val.
     */
    function pushF3Unique( $key, $val, $strict = false) {
        if( ! $this->f3->exists( $key, $oldArr)) {
            $f3->set( $key, []);
            $oldArr = [];
        }
        if( in_array( $val, $oldArr, $strict)) {
            return $val;
        }
        return $this->f3->push( $key, $val);
    }

// --- Utilities for generating random values --------------------------------

    /**
     * Convert openssl_random_pseudo_bytes() to a $length string of hex
     * digits.
     */
    function randomHex( $length) {
        $result = "";
        $rnd = openssl_random_pseudo_bytes( (int) ( ( $length + 1) / 2));
        for( $ix = 0; $ix < strlen( $rnd); $ix++) {
            $bytes = dechex( ord( $rnd[ $ix]));
            if( 1 == strlen( $bytes)) {
                $result .= "0";
            }
            $result .= $bytes;
        }
        if( $length % 2) {
            $result = substr( $result, 0, -1);
        }
        return $result;
    }
}

class GeneralUtilsSingleton extends \Prefab {
    use GeneralUtils;

    function __construct() {
        $this->f3 = \Base::instance();
    }

    private $f3;
}
