<?php

namespace VicocomoTest;

/**
 * Some function that each return a random test values.
 *
 * Needs \Vicocomo\GeneralUtils, a using class must use that, too.
 */
trait RandomValues {

    /**
     * Readonly!
     */
    public $alphanumeric =
      "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    public $aplhanumLc   = "abcdefghijklmnopqrstuvwxyz0123456789";
    public $typeNames    = [ "null", "bool", "int", "string", "pos", "assoc"];
    public $scalarNames  = [ "null", "bool", "int", "string"];

    /**
     * Return a random alphanumeric string.
     *
     * $options is an array of named parameters:
     *
     *   'len':     The length of the returned string, default 20.
     *
     *   'ws':      Controls whitespace in the returned string.
     *                - If it is an array [ <character> => frequency, ...],
     *                  the given characters occur randomly with about the
     *                  given frequency.  They can actually be any characters,
     *                  not only whitespace.
     *                - Otherwise, if truthy, as if [ "\n" => 60, " " => 7].
     *                - Otherwise no whitespace.
     *
     *   'minLen':  If present the length will be minLen <= length <= len.
     *              If not, all strings will be of equal length.
     *
     *   'lowercase-only':
     *              No uppercase characters returned.
     *
     *   'character-set':
     *              NOTE! Single byte characters only!!! The returned
     *              characters are chosen from the string, "lowercase-only" is
     *              ignored.
     *
     * If options is not an array, it is supposed to be a len value.
     */
    function randomAlphanumeric($options = 20) {
        if (is_array($options)) {
            $len           = $this->getArrayVal('len',            $options);
            $ws            = $this->getArrayVal('ws',             $options);
            $minLen        = $this->getArrayVal('minLen',         $options);
            $characterSet  = $this->getArrayVal('character-set',  $options,
                                                                       false);
            $lowercaseOnly = $this->getArrayVal('lowercase-only', $options,
                                                                       false);
            if (!$characterSet) {
                if ($lowercaseOnly) {
                    $characterSet = $this->aplhanumLc;
                } else {
                    $characterSet = $this->alphanumeric;
                }
            }
            $result = '';
            if ($ws && (!is_array($ws))) {
                $ws = ["\n" => 60, ' ' => 7];
            }
            if ($minLen && ($minLen < $len)) {
                $len = rand($minLen, $len);
            }
        } else {
            $len = $options;
            $ws = $minLen = null;
            $characterSet = $this->alphanumeric;
        }
        $lastCharacterSetIx = strLen($characterSet) - 1;
        if (!$len) {
            $len = 20;
        }
        $result = '';
        for ($i = 0;$i < $len;$i++) {
            if ($ws) {
              $gotWs = false;
              foreach ($ws as $wsChr => $freq) {
                  if (1 === rand(1, $freq)) {
                      $result .= $wsChr;
                      $gotWs = true;
                      break;
                  }
              }
              if ($gotWs) {
                  continue;
              }
            }
            $result .= substr($characterSet, rand(0, $lastCharacterSetIx), 1);
        }
        return $result;
    }

    /**
     * Returns a random value according to $options, which is an assocative
     * array of options.  Currently recognized:
     *
     *   "type":         An optional type name from the property $typeNames,
     *                   or an array of them.  Default all of them.
     *
     *   "nest", "min",  Used only if the value happens to be an array, see
     *   "max", "len":   randomArray().
     */
    function randomValue( $options = []) {
        $types = $this->ensureArray(
                $this->getArrayVal( "type", $options, $this->typeNames)
            );
        unset( $options[ "type"]);
        $type = $types[ rand( 0, count( $types) - 1)];
        $opts = [];
        switch( $type) {
        case "null":    return null;
        case "bool":    return (bool) rand( 0, 1);
        case "int":     return rand( -999, 999);
        case "string":  return $this->randomAlphanumeric();
        case "assoc":
            $options [ "assoc"] = true;
            // sic!
        case "pos":
            return $this->randomArray( $options);
        }
    }

    /**
     * Returns a random array according to $options, which is an assocative
     * array of options.  Currently recognized:
     *
     *   "assoc":  The array's top level has only keys that are strings of
     *             1 - 10 lowercase ASCII characters.  Default integer keys.
     *
     *   "nest":   An integer limiting nesting depth, default true meaning
     *             unlimited.  (There is a 33% probability that an element is
     *             an array, so an error is possible but very improbable!)
     *
     *   "min":    The minimum array length, default zero.
     *
     *   "max":    The maximum array length, default 5.
     */
    function randomArray( $options = []) {
        $assoc      = $this->getArrayVal( "assoc", $options);
        $nest       = $this->getArrayVal( "nest",  $options, true);
        $min        = $this->getArrayVal( "min",   $options, 0);
        $max        = $this->getArrayVal( "max",   $options, 5);
        $len        = rand( $min, $max);
        if( $nest && is_numeric( $nest)) {
            $options[ $nest] = $nest - 1;
        }
        $result = [];
        for( $ix = 0; $ix < $len; $ix++) {
            $elem = $nest
                ? $this->randomValue( $options) : $this->randomScalar();
            if( $assoc) {
                do {
                    $key = $this->randomAlphanumeric( [
                            "len" => rand( 1, 10),
                            "character-set" => "abcdefghijklmnopqrstuvwxyz"
                        ]
                    );
                } while( isset( $result[ $key]));
                $result[ $key] = $elem;
            } else {
                $result[] = $elem;
            }
        }
        return $result;
    }

    /**
     * Returns a string representing a random date from Jan 1, 1 to Dec 31,
     * 9999, inclusive.  The string is created by DateTime->format( $format).
     */
    function randomDate($format = 'Y-m-d') {
        if (!$this->$_annoDomino0001) {
            $this->$_annoDomino0001 = gregoriantojd(1, 1, 1);
        }
        if (!$this->$_annoDomino9999) {
            $this->$_annoDomino9999 = gregoriantojd(12, 31, 9999);
        }
        return (
                new \DateTime(
                    jdtogregorian(
                        rand(
                            $this->$_annoDomino0001,
                            $this->$_annoDomino9999
                        )
                    )
                )
            )->format($format);
    }
    private $_annoDomino0001 = null;
    private $_annoDomino9999 = null;
}

