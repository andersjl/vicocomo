<?php

$vicocomo = \Vicocomo\Base::instance();
$f3       = $vicocomo->f3;

if( ! preg_match('#^/test#', $f3->get("PATH"))) {
    $vicocomo->createSqlModelFactories( $f3->get( "vicocomoPoc.models"));
}

