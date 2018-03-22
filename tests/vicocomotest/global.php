<?php

/**
 * "Globalizing" functions in \VicocomoTest\Utils just to simplify writing and
 * reading test scripts.  If you have any problems - e.g. psychological -
 * using global functions, please just use the Utils singleton.  Apart from
 * possible global name clashes there should be no functional difference.
 *
 * For documentation, see the class and the traits it uses.
*/

function factory( $model) {
    return \Vicocomo\Base::instance()->sqlModelFactory( $model);
}

function initTest() {
    return \VicocomoTest\Utils::initTest();
}

function clearDb() {
    \VicocomoTest\Utils::instance()->clearDb();
}

function mock( $request, $params = null, &$responseCode = null) {
    return \VicocomoTest\Utils::instance()
        ->mock( $request, $params, $responseCode);
}

function captureError( $func) {
    return \VicocomoTest\Utils::instance()->captureError( $func);
}

function results() {
    return \VicocomoTest\Utils::instance()->results();
}

function passed() {
    return \VicocomoTest\Utils::instance()->passed();
}

function notes() {
    return \VicocomoTest\Utils::instance()->notes();
}

function storeResult( $status, $text, $extra = null) {
    return
        \VicocomoTest\Utils::instance()->storeResult( $status, $text, $extra);
}

function expect( $test, $text, $onlyOnError = false) {
    return
        \VicocomoTest\Utils::instance()->expect( $test, $text, $onlyOnError);
}

function expects( $summary, $tests, $onlyOnError = false) {
    return
    \VicocomoTest\Utils::instance()->expects( $summary, $tests, $onlyOnError);
}

function message( $msg) {
    return \VicocomoTest\Utils::instance()->message( $msg);
}

function getPrivate($obj, $name) {
    return \VicocomoTest\Utils::instance()->getPrivate($obj, $name);
}

function equal( $a, $b) {
    return \VicocomoTest\Utils::instance()->equal( $a, $b);
}

function castObject( $obj) {
    return \VicocomoTest\Utils::instance()->castObject( $obj);
}

function testMsgFix( $msg, $html = false) {
    return \VicocomoTest\Utils::instance()->testMsgFix( $msg, $html);
}

function testLog() {
    return call_user_func_array(
            [ \VicocomoTest\Utils::instance(), "testLog"], func_get_args()
        );
}

function testExpect() {
    return call_user_func_array(
            [ \VicocomoTest\Utils::instance(), "testExpect"], func_get_args()
        );
}

function orderedAttr( $attr) {
    return call_user_func(
            [ \VicocomoTest\Utils::instance(), "orderedAttr"], $attr
        );
}

function randomAlphanumeric( $options = 20) {
    return \VicocomoTest\Utils::instance()->randomAlphanumeric( $options);
}

function randomValue( $options = null) {
    return \VicocomoTest\Utils::instance()->randomValue( $options);
}

function randomArray( $options = null) {
    return \VicocomoTest\Utils::instance()->randomArray( $options);
}

function randomDate( $format = 'Y-m-d') {
    return \VicocomoTest\Utils::instance()->randomDate( $format);
}

function randomUniqueIntegerFactory( $min, $max) {
    return \VicocomoTest\Utils::instance()
        ->randomUniqueIntegerFactory( $min, $max);
}

function orderedIntegerFactory( $start, $increment = 1) {
    return \VicocomoTest\Utils::instance()
        ->orderedIntegerFactory( $start, $increment);
}

function randomUniqueStringFactory( $opts = []) {
    return \VicocomoTest\Utils::instance()->randomUniqueStringFactory( $opts);
}

function orderedStringFactory( $start, $increment = 1) {
    return \VicocomoTest\Utils::instance()
        ->orderedStringFactory( $start, $increment);
}

function randomDateFactory( $format = 'Y-m-d') {
    return \VicocomoTest\Utils::instance()->randomDateFactory( $format);
}

function randomUniqueDateFactory( $format = 'Y-m-d') {
    return \VicocomoTest\Utils::instance()->randomUniqueDateFactory( $format);
}

