<?php

namespace VicocomoTest;

class Test {

    const NO_EXPECTATIONS = '905c951b712d6c2ed9955079c467790267eef9';

    public $f3;
    public $vicocomo;
    public $utils;

    function __construct($f3) {
        $this->f3       = $f3;
        $this->vicocomo = \Vicocomo\Base::instance();
        $this->utils    = \VicocomoTest\Utils::instance();
    }

    function phpinfo() {
        phpinfo();
    }

    function suite() {
        $this->utils->clearDb();
        $appRoot = $this->f3->get("vicocomoTest.appRoot");
        $suite = $appRoot . "specs/";
        require_once ($appRoot . "vicocomotest/global.php");
        require_once ($appRoot . "phpquery/phpquery.php");
        if ($this->f3->exists("PARAMS.dir", $dir)) {
            $suite .= $dir . "/";
            if ($this->f3->exists("PARAMS.subdir", $subdir)) {
                $suite .= $subdir . "/";
                if ($this->f3->exists("PARAMS.file", $file)) {
                    $suite .= $file . "/";
                }
            }
        }
        $this->f3->set("vicocomoTest.testing", true);
        if (null !== $this->f3->get("GET.loud")) {
            $this->utils->enableQuiet = false;
        }
        list($results, $passed, $notes) = $this->utils->runTest($suite);
        $this->f3->set("vicocomoTest.testSuite", $suite);
        $this->f3->set("vicocomoTest.testResults", $results);
        $this->f3->set("vicocomoTest.passed", $passed);
        $this->f3->set("vicocomoTest.testNotes", $notes);
        header( "Content-Type: text/html", true);
        echo (new \Template)->render("results.htm");
    }
}

