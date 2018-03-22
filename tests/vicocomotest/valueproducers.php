<?php

namespace VicocomoTest;

/*
 * Some function that each return a function thar can be used ro produce
 * random test values with specific properties (unique, ordered, ...).
 *
 * Needs \VicocomoTest\RandomValues and \Vicocomo\GeneralUtils, a using class
 * must use those, too.
*/
trait ValueProducers {

    function randomUniqueIntegerFactory($min, $max) {
        return function () use ($min, $max) {
                static $used = [];
                if (count($used) > $max - $min) {
                    return null;
                }
                do {
                    $result = $min + rand(0, $max - $min);
                } while (isset($used['#' . $result]));
                $used['#' . $result] = true;
                return $result;
            };
    }

    function orderedIntegerFactory($start, $increment = 1) {
        return function () use ($start, $increment) {
                static $current = null;
                if ($current === null) {
                    $current = $start - $increment;
                }
                return $current += $increment;
            };
    }

    /**
     * Return a function that produces random unique strings.
     *
     * $options is an array of named parameters:
     *   'base': Always present in the returned string.
     *           Default ''.
     *   'len', 'ws', 'minLen', 'lowercase-only'
     *           See randomAlphanumeric().
     */
    function randomUniqueStringFactory($options = []) {
        $base = $this->getArrayVal('base', $options, '');
        unset($options['base']);
        if (!$this->getArrayVal('len', $options)) {
            $options['len'] = $base ? strlen($base) : 5;
        }
        return function () use ($base, $options) {
            static $used = [];
            do {
                $result = $this->randomAlphanumeric($options);
                if ($base) {
                    $result = rand(0, 1) ? $result . $base : $base . $result;
                }
            } while (isset($used[$result]));
            $used[$result] = true;
            return $result;
        };
    }

    function orderedStringFactory($start, $increment = 1) {
        return function () use ($start, $increment) {
            static $current = null;
            if ($current === null) {
                $current = $start;
            } else {
                if ($current == $start) {
                    $overflow = true;
                    $next = 0;
                } else {
                    $next = strpos($this->alphanumeric, substr($current, -1))
                            + $increment;
                    if ($next > 61) {
                        $next -= 62;
                        $overflow = true;
                    } else {
                        $overflow = false;
                    }
                }
                $nextChr = substr($this->alphanumeric, $next, 1);
                if ($overflow) {
                    $current .= $nextChr;
                } else {
                    $current = substr_replace($current, $nextChr, -1, 1);
                }
            }
            return $current;
        };
    }

    function randomDateFactory($format = 'Y-m-d') {
        return function () use ($format) {
            return $this->randomDate($format);
        };
    }

    function randomUniqueDateFactory($format = 'Y-m-d') {
        return function () use ($format) {
                static $used = [];
                do {
                    $result = $this->randomDate($format);
                } while (isset($used[$result]));
                $used[$result] = true;
                return $result;
            };
    }
}

