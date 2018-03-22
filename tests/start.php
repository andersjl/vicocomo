<?php

// load Fat-Free Framework unless loaded
require_once ("f3/base.php");
$f3 = \Base::instance();

// Configure Fatfree Framework
$appRoot = __DIR__ . "/";
$f3->set("vicocomoTest.appRoot", $appRoot);
$f3->set("AUTOLOAD", $appRoot);
$f3->set("UI", $appRoot . "templates/");

// Configure and initialize the application
$f3->config($appRoot . "config/application.cfg");
require_once 'config/db.php';
require_once 'config/initialize.php';
$f3->run();

