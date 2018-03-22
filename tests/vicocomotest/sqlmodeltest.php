<?php

namespace VicocomoTest;

class SqlModelTest {
    use \Vicocomo\GeneralUtils;
    use RandomValues;

    public $factory;
    public $required;
    public $vicocomo;
    public $utils;

    /**
     * $modelName is used by $vicocomo->sqlModelFactory().
     *
     * $required is an array (<field> => <function producing a value>, ...) of
     * fields that must not be empty and value producing functions that
     * produce valid values such as a default value or a unique value for each
     * call.  There are several methods in $utils to use for that.
     *
     * If a field in $required has a value that is not a function (e.g. true)
     * the field is still required but must be supplied by the user, see e.g.
     * the check... methods.
     */
    function __construct( $modelName, $required = null) {
        $this->vicocomo = \Vicocomo\Base::instance();
        $this->utils    = \VicocomoTest\Utils::instance();
        $this->factory  = $this->vicocomo->sqlModelFactory( $modelName);
        $this->required = is_array( $required) ? $required : [];
        $this->f3       = \Base::instance();
    }

    /**
     * Create a model object ensuring required fields and optionally store it.
     *
     * If $params is an array of fields and values the object is stored in the
     * database unless the key "do-not-store" is present in $params.  If it
     * is, its value is ignored.
     *
     * If storing fails, returns FALSE.  Also, if the key "report-errors" is
     * present in $params, log an error using $this->utils->storeResult.
     *
     * In any case any callable values will be called to produce the field
     * value used.
     *
     * If $params is an integer or a string, it is taken as the id of a stored
     * object that is retrieved from the database.
     *
     * Returns the created or loaded model object or false if a required field
     * is not in $params and has no default value producing function.
     */
    function createObject( $params = null) {
        $errors = false;
        if( $params && ! is_array( $params)) {
            $store = false;
        } else {
            if( ! $params) {
                $params = [];
            }
            if( isset( $params[ "do-not-store"])) {
                $store = false;
                unset( $params[ "do-not-store"]);
            } else {
                $store = true;
            }
            if( $store) {
                if( isset( $params[ "report-errors"])) {
                    $reportErrors = true;
                    unset( $params[ "report-errors"]);
                } else {
                    $reportErrors = false;
                }
            }
            foreach( $params as $field => & $value) {
                if( $this->callable( $value)) {
                    $value = $value();
                }
            }
            $params = $this->ensureRequired( $params);
            if( ! $params) {       // it seems that the F3 SQL mapper does not
                $errors = "no parameters";  // store a new, empty record?
            }
        }
        if( ! $errors) {
            $result = $this->factory->modelInstance( $params);
            if( $store) {
                $result->store( $errors);
            }
        }
        if( $errors) {
            if( $reportErrors) {
                $this->utils->storeResult(
                    false,
                    "error creating test object:  "
                    . implode( ", ", $this->ensureArray( $errors))
                );
            }
            return false;
        }
        return $result;
    }

    /**
     * Try to store $obj silently.  On error, however, report the errors via
     * $this->utils->storeResult().
     *
     * Returns TRUE if no error, FALSE otherwise.
     */
    function tryStore( $obj) {
        $obj->store( $errors);
        if( $errors) {
            $this->utils->storeResult(
                false,
                "error storing test object:  "
                . implode( ", ", $this->ensureArray( $errors))
            );
            return false;
        }
        return true;
    }

    /**
     * Try to delete $obj silently.  On error, however, report the errors via
     * $this->utils->storeResult().
     *
     * Returns TRUE if no error, FALSE otherwise.
     */
    function tryDelete( $obj) {
        if( ! $obj->delete( $errors)){
            $this->utils->storeResult(
                false,
                "error removing test object:  "
                . implode( ", ", $this->ensureArray( $errors))
            );
            return false;
        }
        return true;
    }

    /**
     * Use the factory to store a number of records.
     *
     * $count is the number of records to store.
     *
     * $params is an optional array of fields and value constants or value
     * producing functions.
     *
     * Ensures required fields
     *
     * Returns an array of the created objects on success, false if a required
     * field is not in $params and has no default value producing function.
     */
    function createObjects( $count, $params = null) {
        $result = [];
        for( $i = 0;$i < $count;$i++) {
            $values = [];
            if( $params) {
                foreach( $params as $field => $value) {
                    if( $this->callable( $value)) {
                        $value = $value();
                    }
                    $values[ $field] = $value;
                }
            }
            $values = $this->ensureRequired( $values);
            if( ! $values) {
                return false;
            }
            $obj = $this->factory->modelInstance( $values);
            if( ! $obj->store()) {
                return false;
            }
            $result[] = $obj;
        }
        return $result;
    }

    /**
     * Clear the test database table $this->tableName.
     */
    function clearTable() {
        $this->vicocomo->getDb()
        ->exec( "DELETE FROM " . $this->factory->tableName);
    }

    /**
     * clearTable(), but first try to call delete() on all stored objects.
     */
    function clearObjects() {
        foreach( $this->factory->find() as $obj) {
            $obj->delete( $ignore, true);
        }
        $this->clearTable();
    }

    /**
     * Count the rows int the test database table $this->tableName.
     */
    function count() {
        return $this->factory->count();
    }

    /**
     * check that a $list of model objects is ordered on $ordered
     *
     * $ordered is either an array of $field names that the objects should be
     * ordered on in that order, or a function of two args that returns truthy
     * iff the args are in correct order.
     *
     * A single field need not be put in an array.
     */
    function isOrdered( $list, $ordered, $desc = false) {
        if( $desc) {
            $list = array_reverse( $list);
        }
        if( ! $this->callable( $ordered) && ! is_array( $ordered)) {
            $ordered =[ $ordered];
        }
        for( $ix = 0;$ix < count( $list) - 1;$ix++) {
            if( $this->callable( $ordered)) {
                if( ! call_user_func( $ordered, $list[ $ix], $list[ $ix + 1])
                ) {
                    return false;
                }
            } else {
                foreach( $ordered as $field) {
                    if( $list[ $ix]->$field < $list[ $ix + 1]->$field) {
                        break;
                    }
                    if( $list[ $ix]->$field > $list[ $ix + 1]->$field) {
                        return false;
                    }
                }
            }
        }
        return true;
    }

    /**
     * Create, try to store model objects with and without a required field,
     * check that they are stored iff it is supplied.
     *
     * The $options array may contain:
     *
     *   "not-required":     An array of fields and values or value producing
     *                       functions in addition to the required fields.
     *                       This seems necessary if is only one required
     *                       field since it seems that F3 refuses to store a
     *                       newly created object with no data.
     *
     *   "extra-producers":  An optional array of required fields and value
     *                       producing functions.  This is needed if a
     *                       required field has no default value producing
     *                       function.
     *
     *   "ensure-valid":     Is used when there are constraints on the
     *                       parameters that a reasonable value producing
     *                       function cannot handle.  If truthy, the protected
     *                       method ensureValid() should be implemented.  It
     *                       will be called before saving this object.
     *   "capture-errors":   Accept error (as opposed to just refusing to
     *                       store the object) if a required field is missing.
     *
     * Does nothing if there are no required fields.
     *
     * Records an error if there is only one required field and no
     * $notRequired-s.
     *
     * Returns nothing.
     */
    function testRequired( $options = []) {
        $notRequired    = $this->getArrayVal( "not-required", $options, []);
        $extraProducers = $this->getArrayVal(
                "extra-producers", $options, []
            );
        $ensureValid    = $this->getArrayVal( "ensure-valid", $options);
        $captureErrors  = $this->getArrayVal( "capture-errors", $options);
        if( ! $this->required) {
            $this->utils->expect( false,
                "the test object must be created with required fields", true
            );
            return;
        }
        if( count( $this->required) + count( $notRequired) <= 1) {
            $this->utils->expect(
                false,
                "need at least two fields to test removing one of them," .
                "\nthe \"not-required\" option may be helpful",
                true
            );
            return;
        }
        $valueProducers = $this->required;
        foreach( $extraProducers as $field => $producer) {
            if( array_key_exists( $field, $valueProducers)) {
                $valueProducers[ $field] = $producer;
            }
        }
        $values = [];
        foreach (
            array_merge( $notRequired, $valueProducers) as $field => $producer
        ) {
            if( $this->callable( $producer)) {
                $values[ $field] = $producer();
            } else {
                $values[ $field] = $producer;
            }
        }
        if( $captureErrors) {
            $obj = null;
            $error = $this->utils->captureError(
                function () use( &$obj, $values, $ensureValid) {
                    $obj = $this->factory->modelInstance( $values);
                    if( $ensureValid) {
                        $obj = $this->ensureValid( $obj);
                    }
                    $obj->store();
                }
            );
        } else {
            $error = false;
            $obj = $this->factory->modelInstance( $values);
            if( $ensureValid) {
                $obj = $this->ensureValid( $obj);
            }
            $obj->store();
        }
        $this->utils->expect(
            ! $error && $obj && $obj->id,
            "required fields suffice to store model object"
        );
        foreach( array_keys( $this->required) as $field) {
            $used = $values;
            unset( $used[ $field]);
            if( $captureErrors) {
                $error = $this->utils->captureError(
                    function () use( $used, &$obj) {
                        $obj = $this->factory->modelInstance( $used);
                        $obj->store();
                    }
                );
            } else {
                $error = false;
                $obj = $this->factory->modelInstance( $used);
                $obj->store();
            }
            $this->utils->expect( $error || ! $obj->id,
                $field . " required to store model object"
            );
        }
    }

    /**
     * Create, try to store model objects with and without a disallowed field,
     * check that they are stored iff it is not supplied.
     *
     * $disallowed is an array of disallowed fields and values
     *
     * $params is an optional array of fields and value constants or value
     * producing functions.
     *
     * Returns nothing.
     */
    function testDisallowed( $disallowed, $params = []) {
        $values = [];
        foreach( array_merge( $this->required, $params)
            as $field => $producer
        ) {
            if( $this->callable( $producer)) {
                $values[ $field] = $producer();
            } else {
                $values[ $field] = $producer;
            }
        }
        $obj = $this->factory->modelInstance( $values);
        $obj->store();
        $this->utils->expect( $obj->id, "no disallowed fields -> store OK");
        foreach( array_keys( $disallowed) as $field) {
            $used = $values;
            $used[ $field] = $disallowed[ $field];
            $obj = $this->factory->modelInstance( $used);
            $obj->store();
            $this->utils->expect( ! $obj->id, $field . " set -> not stored");
        }
    }

    /**
     * Create, store, load a model object, check that the given values
     * persist.
     * And that find() finds all stored objects.
     * And that trying to load a model object with an ID that has never been
     * stored returns NULL.  (As opposed to F3 yielding an empty object.)
     *
     * $testVals is an array (field => value, ...) of values to test the
     * persistence of.  $utils->expect() is called for each value.
     *
     * $generators is an array passed to createObjects() as parameters when
     * creating the objects to store for find() to find.
     *
     * Supplies missing values for required fields that have default value
     * producing functions.
     *
     * Returns nothing.
     */
    function testPersistence( $testVals, $generators = null) {
        $obj = $this->createObject( $this->createObject( $testVals)->id);
        foreach( $testVals as $field => $value) {
            $this->utils->storeResult(
                $this->utils->equal( $obj->$field, $value),
                $field . " is persistent",
                var_export( $obj->$field, true)
                . " != " . var_export( $value, true)
            );
        }
        $countBeforeCreate = $this->count();
        $this->createObjects( 3, $generators);
        $this->utils->storeResult(
            $countBeforeCreate + 3 == count(
                $this->createObject(
                    array_merge( ["do-not-store" => true], $generators ? : [])
                )->factory->find()
            ),
            "find() retrieves all stored objects"
        );
        $this->utils->expect(
            null === $this->createObject( 4711 + 666),
            "loading non-existing ID returns NULL"
        );
    }

    /**
     * Create, store, load with no data and check default values.
     *
     * $defaults is an array <field> => <expected default value>.
     * $utils->expect() is called for each value.
     *
     * $required is an optional array of required fields and values.  This is
     * needed if there are required fields without value producing functions.
     *
     * Supplies missing values for required fields if there is a value
     * producing function.
     *
     * Returns nothing.
     */
    function testDefault( $defaults, $required = null) {
        $obj  = null;
        $that = $this;
        $this->utils->captureError(
            function() use( &$obj, $that, $required) {
                $obj = $that->createObject(
                        $that->createObject( $required)->id
                    );
            }
        );
        $expects = [];
        $fields  = [];
        foreach( $defaults as $field => $value) {
            $fields[] = $field;
            $expects[ "$field is default $value (" . gettype( $value) . ")"]
                = $obj && $obj->$field == $value;
        }
        $this->utils->expects(
            "default " . $this->_pluralS( "value", $fields) . " for "
            . implode( ", ", $fields),
            $expects, $this->utils->testMsgFix( $obj)
        );
    }

    /**
     * Check that duplicate unique fields cannot be stored.
     *
     * $uniques is an array
     *   [ <field> =>[ <sample value>, <alternate value>], ...] of fields
     * that must be unique.  Uniqueness for a combination of fields is
     * indicated by a nested array
     *   [ ..., [
     *       <field 1> =>[ <sample value 1>, <alternate value>],
     *       <field 2> => <sample value 2>, ...
     *     ], ...
     *   ]
     *
     * The alternate value is needed to test that we can store one object with
     * sample value and another with the alternate but get an error when
     * trying to change the alternate value to the sample.  (Yes, this has
     * turned out to be necessary!)
     *
     * Note that the test succeeds if Standard->store() returns one or more
     * errors.  We do rely on this meaning that the record is not stored to
     * the database.  A UNIQUE INDEX is recommended as a saftety belt.
     *
     * $required is an optional array of required fields and values.  This is
     * needed if there are required fields without value producing functions.
     *
     * Clears the database table before and after!
     *
     * The checks are by \Orjhlab\Test->storeResult().  Finally
     * $utils->expect() is run quietly to restore test fixtures.
     *
     * Supplies required fields not present in $uniques.
     *
     * Returns nothing.
     */
    function testUnique( $uniques, $required = []) {
        $expects   = [];
        $allNames = [];
        foreach( $uniques as $n => $fields) {
            $params = $required;
            $altern = $required;
            if( ! is_numeric( $n)) {
                $fields =[ $n => $fields];
            }
            $fieldNames = array_keys( $fields);
            $allNames = array_merge( $allNames, $fieldNames);
            $name = $this->_listFields( $fieldNames);
            $first = true;
            foreach( $fields as $field => $value) {
                if( $first) {
                    $params[ $field] = $value[ 0];
                    $altern[ $field] = $value[ 1];
                    $first = false;
                } else {
                    $params[ $field] = $value;
                    $altern[ $field] = $value;
                    $first = false;
                }
            }
            $this->clearObjects();
            $this->createObject( $params);
            // create a duplicate and try to store it
            $dup = $this->createObject(
                    array_merge( $params, [ "do-not-store" => true])
                );
            $expects[ "duplicate " . $name . " cannot be created"]
                = ! $dup->store();
            // create a non-duplicate and try to change it to a duplicate
            $dup = $this->createObject( $altern);
            if( ! $dup) {
                $expects[ "create from alternate $name"] = false;
                continue;
            }
            $dup->setAttrs( $params);
            $expects[ "Existing $name cannot be changed to duplicate"]
                = ! $dup->store();
        }
        $this->clearObjects();
        // this is NOT a no-op, it restores test fixtures
        $this->utils->expects(
            "unique field " . $this->_pluralS( "value", $allNames) . " for "
            . implode( ", ", $allNames),
            $expects
        );
    }

    /**
     * Test that JSON attributes work as specified.
     *
     * $jsonAttr is a JSON attribute name or an array of them.
     *
     * For each name we test that NULL, FALSE, TRUE, an integer, a string,
     * a positional array, and an associative array can be assigned, stored,
     * and retrieved.  The arrays are random nested.
     */
    function testJsonAttr( $jsonAttr) {
        $obj = $this->createObject( [ "report-errors" => true]);
        $expects = [];
        foreach( $this->ensureArray( $jsonAttr) as $attr) {
            foreach( $this->typeNames as $type) {
                $val = $this->randomValue( [ "type" => $type]);
                $obj->$attr = $val;
                if( ! $this->tryStore( $obj)) {
                    continue;
                }
                $expects[ $type]
                    = $val == $this->createObject( $obj->id)->$attr;
            }
            $this->utils->expects( "store/retrieve $attr", $expects);
        }
        $this->tryDelete( $obj);
    }

    /**
     * Check that sorted() works as expected for simple cases.
     *
     * $ordered is either an array [ <name> => <type or factory>, ...] of
     * attributes that should be sorted or a sort check function.
     *
     *   Attributes:  If the value is one of the recognized types,
     *                currently "date", "int" and "string", random unique
     *                values suitable for that attribute are produced
     *                automatically.
     *
     *                Otherwise, if the value is callable, it should be a
     *                function with no arguments that produces random unique
     *                values.
     *
     *                The sorting is checked using $this->isOrdered(),
     *                descending if $options[ "desc"] is set and truthy.
     *
     *   Callable:    The sorting function should take two model objects and
     *                return true if the order is correct.
     *
     *                $options[ "text"] is a required text describing
     *                $ordered.
     *
     *                $options[ "factory"] is an optional callable that is an
     *                object factory.
     *
     *                If no "factory" given, $options[ "extra"] is an optional
     *                associative array of extra parameters to createObject().
     *
     * Clears the database table before and after!
     */
    function testSorted( $ordered, $options = []) {
        $this->clearTable();
        if( is_array( $ordered)) {
            $check = array_keys( $ordered);
            $descr = implode( ", ", $check);
            $factories = [];
            foreach( $ordered as $name => $type) {
                switch( $type) {
                case "date":
                    $factory = $this->utils->randomUniqueDateFactory();
                break;
                case "int":
                case "integer":
                    $factory
                        = $this->utils->randomUniqueIntegerFactory( -17, 17);
                break;
                case "string":
                    $factory = $this->utils->randomUniqueStringFactory(
                           [ "len" => 5, "lowercase-only" => true]
                        );
                break;
                default:
                    if( $this->callable( $type)) {
                        $factory = $type;
                    } else {
                        $factory = function () {
                                return null;
                            };
                    }
                }
                $factories[ $name] = $factory;
            }
        } else {
            $check = $ordered;
            $descr = $options[ "text"];
        }
        for( $i = 0;$i < 7;$i++) {
            if( is_array( $ordered)) {
                $this->createObject(
                    array_map(
                        function( $factory) {
                            return call_user_func( $factory);
                        }, $factories
                    )
                );
            } else {
                $factory = $this->getArrayVal( "factory", $options);
                if( $factory) {
                    call_user_func( $factory);
                } else {
                    $this->createObject(
                        $this->getArrayVal( "extra", $options)
                    );
                }
            }
        }
        $shouldBeSorted = $this->factory->sorted();
        $desc = $this->getArrayVal( "desc", $options);
        $this->utils->expect(
            $this->isOrdered( $shouldBeSorted, $check, $desc),
            "sorted() orders on " . $descr .( $desc ? " descending" : ""),
            var_export(
                array_map(
                    function( $obj) {
                        return $obj->cast();
                    }, $shouldBeSorted
                ), true
            )
        );
        $this->clearTable();
    }

    /**
     * Check that belongs-to properties are persistent.
     *
     * $belongsTo is an associative array
     *   [ <property name> =>
     *       [ <remote SqlModelTest instance>, <associative argument array>]
     *   , ...
     *   ]
     *
     * NOTE!  If this model has more than one belongs-to association to the
     * same model, you must make separate calls for each of them.
     *
     * The argument arrays in $belongsTo as well as $thisVals is e.g. to
     * provide required fields that have no default value producing functions.
     *
     * $ensureValid is used when there are constraints on the remote objects.
     * If true, the protected method ensureValid() should be implemented.  It
     * will be called before saving this object.
     *
     * Returns nothing.
     */
    function testBelongsTo( $belongsTo, $thisVals = [], $ensureValid = false
    ) {
        $storedObj = $this->createObject( $thisVals);
        $remoteCast = [];
        foreach( $belongsTo as $name => $remote) {
            $remoteTest = $remote[ 0];
            $remoteVals = isset( $remote[ 1]) ? $remote[ 1] : [];
            $remoteObj = $remoteTest->createObject( $remoteVals);
            $storedObj->$name = $remoteObj;
        }
        if( $ensureValid) {
            $storedObj = $this->ensureValid( $storedObj);
        }
        foreach( $belongsTo as $name => $remote) {
            $remoteCast[ $name] = $storedObj->$name->cast();
        }
        $storedObj->store();
        $loadedObj = $this->createObject( $storedObj->id);
        foreach( $belongsTo as $name => $remote) {
            $this->utils->expect(
                $loadedObj->$name->cast() == $remoteCast[ $name],
                $name . " is persistent",
                "expected: " . var_export( $remoteCast[ $name], true)
                . PHP_EOL . "got: "
                . var_export( $loadedObj->$name->cast(), true)
            );
        }
    }

    /**
     * Check overloaded functions and properties for has-many associations.
     *
     * $hasMany is an associative array
     * [ <name> =>
     *     [ "remote-test"    => <SqlModelTest instance for the remote model>
     *     , "on-delete"      => <optional, see SqlModelFactory>
     *     , "ordered-remote" => <optional callback to check remote sorting>
     *     , "remote-vals"    => <optional associative argument array>
     *     , "join-test"      => <SqlModelTest instance for optional join mdl>
     *     , "ordered-join"   => <optional callback to check join sorting>
     *     , "join-vals"      => <optional args to the join model>
     *     ]
     * , ...
     * ]
     *
     * The ...-vals argument arrays in $hasMany as well as
     * $options[ "this-vals"] (see below) is e.g. to provide required
     * attributes that have no default value producing functions. The values
     * may be constant or value producing functions.
     *
     * $options is an associative array of options, currently
     *   "this-vals":      See above.
     *   "do-not-delete":  Do not delete child records even if "cascade".
     *
     * The signature of the ordered...() callbacks is (obj1, obj2): bool, and
     * they should return true if obj2 is not less than obj1.
     *
     * Checks and records using $utils->storeResult that
     *   - the find<name> function and read-only property find all associated
     *     remote objects.
     *   - if "ordered-remote", the sorted<name> function and read-only
     *     property sorts accordingly.
     *   - if "join-test" and "ordered-join", the find<name> function and
     *     read-only property sort accordingly.
     *   - when this is deleted
     *     - "cascade" children and, if "join-test", join table entries are
     *       deleted unless $options[ "do-not-delete"]
     *     - "restrict" or "set-null" children are not deleted
     *     - foreign keys in "set-null" children are set to null
     *
     * Finally $utils->expect() is run quietly to restore test fixtures.
     */
    function testHasMany( $hasMany, $options = []) {
        $storedObj = $this->createObject(
                $this->getArrayVal( "this-vals", $options)
            );
        $thisId = $storedObj->id;
        // prepare the $hasMany entries
        foreach( $hasMany as $name => & $opts) {
            $opts = $this->ensureKey( [
                        [ "on-delete", "restrict"], "ordered-remote",
                        [ "foreign-key", "fk" . $this->factory->modelName],
                        [ "remote-vals", []], "join-test"
                    ], $opts
                );
            if( $opts[ "join-test"]) {
                $opts = $this->ensureKey( [
                            "ordered-join", [ "join-vals", []],
                            [ "remote-key", "fk$name"]
                        ], $opts
                    );
                $opts[ "on-delete"] = "set-null";
                $opts[ "join-vals"][ $opts[ "foreign-key"]] = $thisId;
            } else {
                $opts[ "remote-vals"][ $opts[ "foreign-key"]] = $thisId;
            }
        }
        unset( $opts);
        // create objects
        $allStoredCasts = [];
        $initialCounts = [];
        foreach( $hasMany as $name => $opts) {
            $initialCounts[ $name] =
                $opts[ "join-test"]
                ? $opts[ "join-test"]->factory->count()
                : $opts[ "remote-test"]->factory->count();
            $storedCasts = [];
            for( $i = 0;$i < 3;$i++) {
                $failed    = "remote";
                $remoteObj = $opts[ "remote-test"]
                    ->createObject( $opts[ "remote-vals"]);
                if( $remoteObj) {
                    $failed = false;
                    $storedCasts[] = $remoteObj->cast();
                    if( $opts[ "join-test"]) {
                        $failed = "join";
                        if( $opts[ "join-test"]->createObject(
                                array_merge(
                                    $opts[ "join-vals"],
                                    [ $opts[ "remote-key"] => $remoteObj->id]
                                )
                            )
                        ) {
                            $failed = false;
                        }
                    }
                }
                if( $failed) {
                    $this->utils->storeResult(
                        false,
                        "testHasMany() create $failed object for association"
                        . " $name"
                    );
                    continue 2;
                }
            }
            sort( $storedCasts);
            $allStoredCasts[ $name] = $storedCasts;
        }
        $loadedObj = $this->createObject( $thisId);
        if( ! $loadedObj) {
            return false;
        }
        // check that the functions and properties find what they should
        foreach( $hasMany as $name => $opts) {
            if( ! isset( $allStoredCasts[ $name])) {
                continue;
            }
            $expects = [];
            $this->_testHasManyDo(
                $expects, $loadedObj, "find" . $name, $allStoredCasts[ $name]
            );
            if( $opts[ "join-test"] && $opts[ "ordered-join"]) {
                $this->_testHasManyDo(
                    $expects, $loadedObj, "sorted" . $name,
                    $allStoredCasts[ $name], [
                        $opts[ "ordered-join"],
                        $opts[ "join-test"]->factory,
                        $thisId,
                        $opts[ "foreign-key"],
                        $opts[ "remote-key"]
                    ]
                );
            } elseif( $opts[ "ordered-remote"]) {
                $this->_testHasManyDo(
                    $expects, $loadedObj, "sorted" . $name,
                    $allStoredCasts[ $name], $opts[ "ordered-remote"]
                );
            }
            $this->utils->expects(
                "one " . $this->factory->modelName . " finds many"
                . $this->_testHasManyText( $opts),
                $expects
            );
        }
        if( ! ( isset( $options[ "do-not-delete"])
                && $options[ "do-not-delete"]
            )
        ) { // check that cascading works
            $restricted = array_reduce(
                    array_keys( $hasMany),
                    function( $acc, $name) use( $hasMany) {
                        return $acc
                            ?: ( "restrict" == $hasMany[ $name][ "on-delete"]
                                ? $name : false
                            );
                    }, false
                );
            $deleted = $loadedObj->delete()
                    && ! $this->createObject( $thisId);
            $this->utils->expect(
                (bool) $restricted != $deleted,
                $this->factory->modelName . " deletion "
                . ( $restricted
                    ? "restricted by $restricted" : "not restricted"
                )
            );
            foreach( $hasMany as $name => $opts) {
                $onDel = $deleted
                    ? ( $opts[ "join-test"] ? "cascade" : $opts[ "on-delete"])
                    : "restrict";
                $remaining = ( $opts[ "join-test"] ?: $opts[ "remote-test"])
                        ->factory->find();
                $expects = [];
                switch( $onDel) {
                case "cascade":
                    $expects[ $name . " objects cascade on delete"]
                        = count( $remaining) === $initialCounts[ $name];
                break;
                case "set-null":
                    $fk = $opts[ "foreign-key"];
                    $expects[
                        "$name->$fk is set to null on "
                        . $this->factory->modelName . " delete"
                    ] = array_reduce(
                            $remaining,
                            function( $a, $o) use( $fk, $thisId) {
                                return $a &&( $o->$fk !== $thisId);
                            },
                            true
                        );
                // sic!
                case "restrict":
                    $expects[
                        $name . " objects survive delete"
                        . ( $restricted ? ' inhibited by "restrict"' : "")
                    ] = count( $remaining) == count( $allStoredCasts[ $name]);
                break;
                }
                $this->utils->expects(
                    "one " . $this->factory->modelName . " deletion affects"
                    . $this->_testHasManyText( $opts),
                    $expects
                );
            }
        }
    }
    private function _testHasManyDo(
        &$expects, $obj, $nam, $sav, $ord = null
    ) { // test the property without join factory ordering
        $this->_testHasManyDoCheck(
            $expects,
            array_map(
                function( $obj) {
                    return $obj->cast();
                }, $obj->$nam
            ), $nam, "property", $sav, is_array( $ord) ? null : $ord
        );
        // test the function without join factory ordering
        $this->_testHasManyDoCheck(
            $expects,
            array_map(
                function( $obj) {
                    return $obj->cast();
                }, call_user_func( [ $obj, $nam])
            ), $nam, "method", $sav, is_array( $ord) ? null : $ord
        );
        // if required (i.e. $ord is an array) test join factory ordering
        if( is_array( $ord)) {
            $this->_testHasManyDoCheck(
                $expects,
                array_map(
                    function( $obj) {
                        return $obj->cast();
                    },
                    call_user_func(
                        [ $obj, $nam], "", [ "join-sort" => true]
                    )
                ), $nam, "method", $sav, $ord
            );
        }
    }
    private function _testHasManyDoCheck(
        &$expects, $got, $nam, $txt, $sav, $ord
    ) {
        $msg = $nam . " " . $txt . " finds " . count( $sav) . " remote";
        $reord = $got;
        sort( $reord);
        $result = $reord === $sav;
        $okBeforeSortCheck = $result;
        if( $result && $ord) {
            $msg .= " in correct order";
            if( is_array( $ord)) {  // awkward way to signal join ordering
                list( $ordFun, $jFac, $thId, $fKey, $rKey) = $ord;
                $jTab = $jFac->tableName;
                $ordList = [];
                foreach( $got as $rem) {
                    $rId = $rem[ "id"];
                    $ordList[] = $jFac->findone(
                            "$fKey = $thId AND $rKey = $rId"
                        )->cast();
                }
            } else {
                $ordFun = $ord;
                $ordList = $got;
            }
            $o2 = $ordList[ 0];
            for( $ix = 1;$ix < count( $ordList);$ix++) {
                $o1 = $o2;
                $o2 = $ordList[ $ix];
                if( ! $ordFun( $o1, $o2)) {
                    $result = false;
                    break;
                }
            }
        }
        $expects[ $msg] = $result;
    }
    private function _testHasManyText( $opts) {
        return " "
            . $opts[ "remote-test"]->factory->modelName
            . ( $opts[ "join-test"]
                ? " (through " . $opts[ "join-test"]->factory->modelName . ")"
                : ( $opts[ "on-delete"]
                    ? " (" . $opts[ "on-delete"] . ")" : ""
                )
            ).( $opts[ "foreign-key"]
                ? ", fk " . $opts[ "foreign-key"] : ""
            ).( $opts[ "join-test"] && $opts[ "remote-key"]
                ? ", rk " . $opts[ "remote-key"] : ""
            );
    }

    protected function ensureValid( $modelObject) {
        return $modelObject;
    }

    protected function ensureRequired( $params) {
        foreach( $this->required as $field => $producer) {
            if( ! array_key_exists( $field, $params)) {
                $params[ $field] = $producer();
            }
        }
        return $params;
    }

    private $f3;

    private function _listFields( $fields) {
        $fields = $this->ensureArray( $fields);
        if( 1 == count( $fields)) {
            return reset( $fields);
        }
        return "(" . implode( ", ", $fields) . ")";
    }

    private function _pluralS( $word, $n) {
        if( is_array( $n)) {
            $n = count( $n);
        }
        return $word .( $n > 1 ? "s" : "");
    }
}

