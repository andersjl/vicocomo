<?php

namespace Vicocomo;

/**
 * A trait for implementing an MVC model object that uses a \DB\SQL\Mapper
 * to handle and finding model data.
 *
 * The mapper is stored in the trait property $mapper.  The class using the
 * trait will in many respects behave like a \DB\SQL\Mapper, since all
 * undefined methods and properties except as described below are sent to
 * the mapper (duck typing).  Note, however, that there is no array access of
 * properties.
 *
 * In addition, SqlModel handles one-to-many and many-to-many associations,
 * text columns storing markdown, JSON, or objects, and object comparison.
 *
 * The model class using the trait has to
 *
 * - define the public static method factoryOptions().  The method should not
 *   take any parameters. It should return the $options parameter to the
 *   SqlModelFactory constructor, see SqlModelFactory.  factoryOptions() is
 *   used by Vicocomo\Base->createSqlModelFactories() to set up the factory.
 *
 * - have a __construct( $factory, $params, $fields, $ttl) that forwards the
 *   parameters to initSqlModel(), see that.
 *
 * If it does so, and the application initialization calls
 * \Vicocomo::instance()->createSqlModelFactories() with the class name, each
 * instance of it will be connected to a class specific SqlModelFactory
 * instance that is responsible for and speeds up model instantiation.  The
 * factory is stored in the trait property $factory.
 *
 * In addition to forwarding methods and properties to the mapper, the trait
 * adds capabilities to the model object as defined by its static method
 * factoryOptions():
 *
 *
 * JSON and object attributes ------------------------------------------------
 *
 * The columns present in $this->factory->jsonAttrs are automatically JSON
 * encoded and decoded, possibly handled by the ObjectAttr interface.  See the
 * "json-attrs" and "object-attrs" options to SqlModelFactory->__construct().
 *
 * If not using ObjectAttr, the property value is converted to a string by
 * json_encode() (with no options) when stored in the database, and converted
 * by json_decode() when retrieved from the database.  When decoding, JSON
 * objects are decoded to PHP associative arrays.  If you need to store and
 * retreive objects, use ObjectAttr.
 *
 * When using ObjectAttr, getting, setting, storing and retreiving an
 * attribute is a collaboration between SqlModel, SqlModelFactory, and the
 * class implementing ObjectAttr.
 *
 * The attribute value may be a single instance of a class implementing the
 * ObjectArray interface or an array of them.  In the latter case, when
 * setting the attribute to an array of PHP values, SqlModel does
 *   array_values( array_filter( <array of to-be PHP values>))
 * before calling the set() method of the existing instances.
 *
 *
 * The "one" side of one-to-many associations --------------------------------
 *
 * For each entry in $this->factory->hasMany with key <name> there are
 * functions new<name>() (not for "through" associations), find<name>(), and
 * sorted<name>().
 *
 * If the association is not "through" a join-table, {find,sorted}<name>()
 * send their arguments on to <remote factory>->{find,sorted}(), but add a
 * filter to return only records that are associated with $this->id.
 *
 * If "through" the filter selects all remote ID's that are associcated to
 * the first parameter via the "through" join table.
 *
 * When "through", sorted...() sorts using the sort() function of the remote
 * model's factory.  To achieve sorting by the join factory instead, set the
 * "join-sort" flag in the second ($options) argument to sorted...().
 *
 * new<remote-name>() takes an associative array as argument.  It adds an
 * entry <remote foreign key column> => $id, and sends the array on to
 * <remote factory>->modelInstance().
 *
 * Note that new...() is not (yet) implemented for "through" associations!
 *
 * Note that this allows for more than one has-many association from $this
 * model to *the same* remote-name.
 *
 * There are also readonly properties {find,sorted}<remote-name> that cache
 * the corresponding function's value.  Before the first call to the function,
 * accessing the property calls the function with no parameters.
 *
 *
 * The "many" side of one-to-many associations -------------------------------
 *
 * For each entry in $this->factory->belongsTo with key <name> there is a
 * read/write property with that name.  The value is the model object that
 * $this belongs to.  When setting this property the primary key attribute
 * must have a value.
 */
trait SqlModel {

    /**
     * READONLY!  The \DB\SQL\Mapper that $this owns.
     */
    public $mapper;

    /**
     * READONLY!  The SqlModelFactory that produced $this.
     */
    public $factory;

    /**
     * Initialize the trait.  Should be called by the using class'
     * __construct().
     *
     * $factory is an instance of SqlModelFactory.  It must be compatible with
     * the using class.  The easy way to ensure this is to instantiate doing
     * aFactory->modelInstance() rather than new UsingClass().
     *
     * $params, see SqlModelFactory->modelInstance().  In addition, if $params
     * is an object it is taken as a \DB\SQL\Mapper to use, none is created,
     * and the $fields and $ttl parameters are ignored.  This is primarily for
     * internal use by SqlModelFactory.
     *
     * $fields and $ttl are forwarded to the mapper constructor.
     */
    function initSqlModel( $factory, $params, $fields, $ttl) {
        if( gettype( $params) === "object") {  // should be a \DB\SQLMapper
            $this->mapper = $params;
            $params = null;
        } else {
            $this->mapper = new \DB\SQL\Mapper(
                    $factory->db, $factory->tableName, $fields, $ttl
                );
        }
        $this->factory = $factory;
        if( $params) {
            $this->setAttrs( $params);
        }
    }

    /**
     * Compare this to other using the factory's compare method.
     */
    function compare( $other) {
        $cmp = $this->factory->compare;
        return $cmp( $this, $other);
    }

    /**
     * A utility to enable checking if a foreign key is stale without
     * declaring a belongs-to asociation.
     *
     * $model is the model name of object that $fk refers to.
     *
     * $fk is the foreign key value in $this.  The column name in  the table
     * must be "id".
     */
    function rowExists( $model, $fk) {
        return $this->factory->factory( $model)->rowExists( $fk);
    }

    /**
     * Set fields from the associative array $params.  "json-attrs" fields
     * defined when creating $this->factory are handled.
     */
    function setAttrs( $params) {
        foreach( $params as $key => $val) {
            $this->_set( $key, $val);
        }
        return $this;
    }

    /**
     * Fill $errors with any errors from collectErrorsPreventingStore().
     *
     * If neither errors nor $dryRun, let ObjectAttr implementations do their
     * thing and store attributes to the database.
     *
     * Returns FALSE if errors, TRUE if none.
     */
    function store( &$errors = null, $dryRun = false) {
        $errors = $this->collectErrorsPreventingStore();
        if( $errors || $dryRun) {
            return ! $errors;
        }
        foreach( $this->factory->jsonAttrs as $attr => $def) {
            $vals = $this->_cacheJsonAttr( $attr, $def);
            if( $vals && $def) {
                if( count( $def) > 1) {
                    $vals = array_map(
                                function( $obj) {
                                    return $obj ? $obj->toStore() : $obj;
                                }, $vals
                              );
                    while( ! end( $vals) && count( $vals) > $def[ 1]) {
                        array_pop( $vals);
                        array_pop( $this->_jsonAttrVals[ $attr]);
                    }
                    reset( $vals);
                } else {
                    $vals = $vals ? $vals->toStore() : $vals;
                }
            }
            $this->mapper->set(
                $attr,
                $vals ? json_encode( $vals, JSON_UNESCAPED_UNICODE) : null
            );
        }
        foreach( $this->factory->attrs as $attr => $def) {
            if( ! is_array( $def)) {
                continue;
            }
            $val = $this->mapper->get( $attr);
            if( ! $val) {
                continue;
            }
            $this->mapper->set( $attr, mb_substr( $val, 0, $def[ 1]));
        }
        $this->mapper->save();
        return true;
    }

    /**
     * Delete $this and recursively "cascade" children.
     *
     * If not $force, check collectErrorsPreventingDelete().
     *
     * If that returns truthy, the return value is put in $offending.  It
     * indicates a problem, see collectErrorsPreventingDelete(), neither the
     * database or $this are changed, and this here function returns FALSE.
     *
     * If $force or collectErrorsPreventingDelete() is falsy:
     *   Handle hasMany associations:
     *     Delete the join rows for all "through" associations.
     *     Delete "cascade" child models recursively.
     *     If $force, also delete "restrict" child models recursively.
     *     Nullify the foreign key in "set-null" child models.
     *   Let the ObjectAttr implementations in $this do their thing.
     *   Delete the database row.
     *   Return TRUE.
     */
    function delete( &$offending = null, $force = false) {
        if( $this->mapper->dry() || ! $this->mapper->get( "id")) {
            return true;
        }
        if( ! $force) {
            $offending = $this->collectErrorsPreventingDelete();
            if( $offending) {
                return false;
            }
        }
        foreach( $this->factory->hasMany as $name => $props) {
            $fk = $props[ "foreign-key"];
            $through = $props[ "through"];
            if( $through) {
                $this->_deleteCascade( $through, $fk, null);
                continue; // ignore "on-delete"
            }
            $remoteModel = $props[ "remote-model"];
            switch( $props[ "on-delete"]) {
            case "restrict":
                if( $force) {
                    $this->_deleteCascade( $remoteModel, $fk, true);
                }
            break;
            case "cascade":
                $this->_deleteCascade( $remoteModel, $fk, $force);
            break;
            case "set-null":
                $this->factory->db->exec(
                    "UPDATE "
                    . $this->factory->factory( $remoteModel)->tableName
                    . " SET $fk = NULL WHERE $fk = $this->id"
                );
            break;
            }
        }
        foreach( $this->factory->jsonAttrs as $attr => $def) {
            $vals = $this->_cacheJsonAttr( $attr, $def);
            if( $vals && $def) {
                if( count( $def) > 1) {
                    foreach( $vals as $obj) {
                      $obj->delete();
                    }
                } else {
                    $vals->delete();
                }
            }
        }
        $this->_assocVals = [];
        $this->mapper->erase();
        return true;
    }
    private function _deleteCascade( $remoteName, $foreignKey, $force) {
        $remoteFactory = $this->factory->factory( $remoteName);
        if( $remoteFactory->view) {
            return;
        }
        foreach( $remoteFactory->find( "$foreignKey = $this->id") as $obj) {
            $obj->delete( $force);
        }
    }

    /**
     * Forget all DB column values except primary keys and return $this.
     */
    function clearAttrs() {
        foreach( array_keys( $this->factory->attrs) as $field) {
            $this->mapper->clear( $field);
        }
        $this->_jsonAttrVals = [];
        $this->_assocVals = [];
        return $this;
    }

    /**
     * $this->mapper->cast(), unpacking JSON attributes.
     *
     * $obj may be any object of a class using SqlModel, default $this.
     */
    function cast( $obj = null) {
        if( ! $obj) {
            $obj = $this;
        }
        $result = $this->mapper->cast( $obj->mapper);
        foreach( array_keys( $obj->factory->jsonAttrs) as $attr) {
            $result[ $attr] = $obj->$attr;
        }
        return $result;
    }

    /**
     * Create a new unstored instance from all attributes except "id".
     */
    function createClone() {
        // We could use the PHP clone statement with a clever __clone()
        // function.  However, a createClone() function is simpler since we do
        // not have to traverse all the ObjectAttrs and clone them.
        $params = [];
        foreach( array_keys( $this->factory->attrs) as $attr) {
            $params[ $attr] = $this->mapper->get( $attr);
        }
        return $this->factory->modelInstance( $params);
    }

    function __call( $name, $args) {
        $assFunList = $this->_assocFunc( $name);
        if( $assFunList) {
            list( $assFun, $func, $assNam)  = $assFunList;
            $assoc  = $this->factory->hasMany[ $assNam];
            $thisId = $this->mapper->get( "id");
            if ( "new" == $func) {
                if( $assoc[ "through"] || ! $thisId) {
                    return null;
                }
                $remPars = array_shift( $args) ? : [];
                $remPars[ $forKey] = $thisId;
                return \Vicocomo::instance()->sqlModelFactory(
                        $assoc[ "remote-model"]
                    )->modelInstance( $remPars);
            }
            if( ! $thisId) {
                return [];
            }
            $through = $assoc[ "through"];
            $sort    = "sorted" == $func;
            if( $sort && $through && 2 <= count( $args)
                && isset( $args[ 1][ "join-sort"])
            ) {
                if( $args[ 1][ "join-sort"]) {
                    $sort = "join-sort";
                }
                unset( $args[ 1][ "join-sort"]);
            }
            array_unshift( $args, $assNam, $sort, $thisId);
            return call_user_func_array( [
                        $this->factory,
                        $through ? "findManyThrough" : "findMany"
                    ], $args
                );
        }
        return $this->mapper->__call( $name, $args);
    }

    function __set( $name, $value) {
        return $this->_set( $name, $value);
    }

    function __get( $name) {
        $assFunList = $this->_assocFunc( $name);
        if( $assFunList) {
            list( $assFun, $func, $assNam)  = $assFunList;
            if( ! isset( $this->_assocVals[ $name])) {
                $this->_assocVals[ $name] = $this->$name();
            }
            return $this->_assocVals[ $name];
        }
        if( isset( $this->factory->belongsTo[ $name])) {
            $def = $this->factory->belongsTo[ $name];
            $fkName = $def[ "foreign-key"];
            $fk = $this->mapper->get( $fkName);
            if( $fk) {
                return $this->factory->factory( $def[ "remote-model"])
                        ->modelInstance( $fk);
            } else {
                return null;
            }
        }
        if( isset( $this->factory->jsonAttrs[ $name])) {
            $val = $this->_cacheJsonAttr( $name);
            if( ! $val) {
                return null;
            }
            $def = $this->factory->jsonAttrs[ $name];
            if( $def) {
                if( count( $def) > 1) {
                    $val = array_map(
                            function( $obj) {
                                return $obj ? $obj->get() : $obj;
                            }, $val
                        );
                } else {
                    $val = $val->get();
                }
            }
            return $val;
        }
        return $this->mapper->get( $name);
    }

    function __toString() {
        $attrs = [ "id" => $this->mapper->get( "id") ];
        foreach( array_keys( $this->factory->attrs) as $attr) {
            $attrs[ $attr] = $this->mapper->get( $attr);
        }
        return var_export( $attrs, true);
    }

    /**
     * Returns an array of error messages if the object cannot be stored in
     * its present state, falsy if it can.
     *
     * The errors are collected from two sources,
     * $this->errorsPreventingStore() and the errorsPreventingStore() method
     * of every object attribute.  Any exceptions thrown are catched and their
     * error messages are added.
     *
     * NOTE that predicted store() success is indicated by a falsy return!!!
     */
    final function collectErrorsPreventingStore() {
        $errors = [];
        try {
            $errors = $this->errorsPreventingStore() ? : [];
            foreach( $this->factory->jsonAttrs as $attr => $def) {
                $vals = $this->_cacheJsonAttr( $attr, $def);
                if( $vals && $def) {
                    if( count( $def) == 1) {
                        $vals = [ $vals];
                    }
                    foreach( $vals as $obj) {
                        if( ! $obj) {
                            continue;
                        }
                        $errors = array_merge(
                                $errors, $obj->errorsPreventingStore() ? : []
                            );
        } } } }
        catch( \Exception $e) {
            $errors = array_push( $errors, $e->getMessage());
        }
        return $errors;
    }

    /**
     * Returns an array indicating what prevents deleting the object in its
     * present state, falsy if it can be deleted.
     *
     * The on-delete = "cascade" SqlModelFactory->hasMany associations
     * recursively define a tree of child model objects.
     *
     * If there is a leaf in that tree that has an on-delete = "restrict"
     * association with existing rows:
     *   Returns an array
     *     [ "restrict"
     *     , [ <"cascade" remote-name>, <child id>], ... (zero or more pairs)
     *     , <"restrict" remote-name>
     *     ]
     *
     * Otherwise if errorsPreventingDelete() returns truthy:
     *   Returns the array of error messages with "error" prepended.
     *
     * NOTE that predicted delete() success is indicated by a falsy return!!!
     */
    final function collectErrorsPreventingDelete() {
        $offending = $this->_deleteRestrict( []);
        if( $offending) {
            array_unshift( $offending, "restrict");
            return $offending;
        }
        $errors = $this->errorsPreventingDelete();
        if( $errors) {
            if( ! is_array( $errors)) {
                $errors = [ $errors];
                array_unshift( $errors, "error");
                return $errors;
            }
        }
        return false;
    }
    /**
     * Not intended for use outside of this file, but cannot be private.
     */
    function _deleteRestrict( $tree) {
        foreach( $this->factory->hasMany as $name => $props) {
            switch( $props[ "on-delete"]) {
            case "cascade":
                $children = "find$name";
                foreach( $this->$children as $child) {
                    array_push( $tree,[ $name, $child->id]);
                    $offending = $child->_deleteRestrict( $tree);
                    if( $offending) {
                        return $offending;
                    }
                }
            break;
            case "restrict":
                $remoteTable = $this->factory->factory(
                        $props[ "remote-model"]
                    )->tableName;
                if( $this->factory->db->exec( "SELECT * from $remoteTable"
                            . "  WHERE " . $props[ "foreign-key"]
                            . " = $this->id"
                        )
                ) {
                    array_push( $tree, $name);
                    return $tree;
                }
                break;
              default:
                  continue;
            }
        }
        return false;
    }

    /**
     * Override to generate a list of error messages indicating any problems
     * with the object state that prevents storing it.
     *
     * Handy protected helper functions (see those):
     *   reportFalsy()
     *   reportRowMising()
     *   reportDuplicate()
     *   reportDuplicateSibling()
     *   checkInteger()
     *   checkDateFormat()
     */
    protected function errorsPreventingStore() {
        return null;
    }

    /**
     * Override to generate a list of error messages indicating any problems
     * with the object state that prevents deleting it.
     */
    protected function errorsPreventingDelete() {
        return null;
    }

    /**
     * Append an error "$prefix ..." to the positional array $errors for each
     * entry in $requiredAttrs that has a falsy value.
     *
     * Each entry in $requiredAttrs is a pair [ <attr name>, <human name>] or
     * a string used for both.
     */
    final protected function reportFalsy( &$errors, $requiredAttrs,
        $prefix = "saknar"
    ) {
        $result = true;
        foreach( $requiredAttrs as $attrName) {
            if( is_string( $attrName)) {
                $attr = $name = $attrName;
            } else {
                $attr = $attrName[ 0];
                $name = $attrName[ 1];
            }
            if( ! $this->$attr) {
                $errors[] = "$prefix $name";
                $result = false;
            }
        }
        return $result;
    }

    /**
     * Append an error "$missing <human name> $withId <foregin key value in
     * this>" to the positional array $errors for each entry in $requiredAttrs
     * where rowExists() is falsy.
     *
     * $requiredFks is a positional array of positional arrays [
     *   <foreign model name>, <human name>, <foreign key name>, <allow null>
     * ]
     * The foreign key name is optional, default fk<foreign model name>.
     * If the last, optional entry is truthy iff a null/zero foreign key is
     * OK.  If not and the foreign key is falsy, error
     * "$missing $refTo <human name".
     */
    final protected function reportRowMissing( &$errors, $requiredFks,
        $missing = "saknar", $refTo = "referens till", $withId = "med id"
    ) {
        $result = true;
        foreach( $requiredFks as list( $mdl, $name, $fkNam, $allowNull)) {
            $fkNam = $fkNam ? : "fk$mdl";
            $fkVal = $this->$fkNam;
            if( $fkVal) {
                if( ! $this->rowExists( $mdl, $fkVal)) {
                    $errors[] = "$missing $name $withId $fkVal";
                    $result = false;
                }
            } elseif( ! $allowNull) {
                $errors[] = "$missing $refTo $name";
                $result = false;
            }
        }
        return $result;
    }

    /**
     * Append an error "$textBefore ... $textAfter" to the positional array
     * $errors iff there is a duplicate stored in the database.
     *
     * $unique is an array of attribute names or pairs of attribute names and
     * human names used in the error messages.  If it is an array, all values
     * must be the same to count as a duplicate.
     *
     * If you have more than one field or field combination that should be
     * uniqe, call reportDuplicate once for each.
     */
    final protected function reportDuplicate( &$errors, $unique,
        $textBefore = "en rad med", $textAfter = "finns redan"
    ) {
        $params = [];
        $names = [];
        foreach( $unique as $field) {
            if( is_string( $field)) {
                $attr = $name = $field;
            } else {
                list( $attr, $name) = $field;
            }
            $names[] = $name;
            $params[ $attr] = $this->$attr;
        }
        $combo = count( $names) != 1;
        $dup = $this->factory->findUnique( $params);
        if( $dup && $dup->mapper->get( "id") != $this->mapper->get( "id")) {
            $errors[] =
                "$textBefore " .( $combo ? "(" : "") . implode( ", ", $names)
                . ( $combo ? ")" : "") . " = " . ( $combo ? "(" : "")
                . implode( ", ", array_values( $params))
                . ( $combo ? ")" : "") . " $textAfter";
            return false;
        }
        return true;
    }

    /**
     * Append an error "$parNam $existsAs $sibNam" to the positional array
     * $errors iff there is a row in in the table $sibTbl with the column
     * $sibFk equal to $parId.
     *
     * If $parId is falsy, error "$missing $parNam".
     */
    final protected function reportDuplicateSibling( &$errors, $parId,
        $parNam, $sibTbl, $sibFk, $sibNam, $existsAs = "finns redan som",
        $missing = "saknar"
    ) {
        if( ! $parId) {
            $errors[] = "$missing $parNam";
            return false;
        }
        if( $this->db->exec(
                "SELECT `id` FROM `$sibTbl` WHERE `$sibFk` = $parId"
            )
        ) {
            $errors[] = "$parNam $existsAs $sibNam";
            return false;
        }
        return true;
    }

    /**
     * Append an error "... ska vara ett heltal, fick ..." to the positional
     * array $errors if motivated.  Leading and trailing spaces are not
     * allowed.  Floats are not allowed.
     *
     * $int is the name of the field that should be an integer.
     *
     * $shown, if given, is shown instead of $name in the error message.
     *
     * If $options[ "default"] is an integer (not a string!) and $int is not
     * integerish, $int is set to $options[ "default"] and $errors is
     * untouched.
     *
     * If $options[ "message"] is a non-blank string it is used instead of
     * "ska vara ett heltal, fick".
     */
    final protected function checkInteger( &$errors, $int, $shown = null,
        $options = []
    ) {
        $val = $this->mapper->get( $int);
        $def = $options[ "default"];
        $msg = trim( $options[ "message"]) ? : "ska vara ett heltal, fick";
        if( ! ( is_int( $val) || ctype_digit( $val))) {
            if( is_int( $def)) {
                $this->$int = $def;
            } else {
                $errors[] = ( $shown ? : $int) . " $msg \"$val\"";
                return false;
            }
        }
        return true;
    }

    /**
     * Append an error
     * "... $hasIllegalDateFormatGot ..., $shouldBe YYYY-MM-DD"
     * to the positional array $errors if motivated.
     *
     * $date is the name of the field to test for date format.
     */
    final protected function checkDateFormat( &$errors, $date,
        $hasIllegalDateFormatGot = "har felaktigt datum, fick",
        $shouldBeAnExistingDate = "ska vara ett existerande datum"
    ) {
        if( ! preg_match( '/(\d\d\d\d)-(\d\d)-(\d\d)/',
                $this->mapper->get( $date), $matches
            ) || ! checkdate( $matches[ 2], $matches[ 3], $matches[ 1])
        ) {
          $errors[] = "$date $hasIllegalDateFormatGot $this->$date"
              . ", $shouldBeAnExistingDate YYYY-MM-DD";
          return false;
        }
        return true;
    }

    private $_assocVals    = [];
    private $_jsonAttrVals = [];

    private function _set( $name, $value) {
        if( isset( $this->factory->belongsTo[ $name])) {
            $this->mapper->set(
                $this->factory->belongsTo[ $name][ "foreign-key"],
                is_object( $value)
                ? $value->mapper->get( "id")
                : ( is_numeric( $value) ? (int) $value : null)
            );
        } elseif( isset( $this->factory->jsonAttrs[ $name])) {
            $def = $this->factory->jsonAttrs[ $name];
            if( $def) {
                $old = $this->_cacheJsonAttr( $name, $def);
                if( count( $def) > 1) {
                    $this->_setObjectAttrArr(
                        $name, $old, $value, $def[ 0], $def[ 1]
                    );
                } else {
                    if( ! $old) {
                        $objectAttrClass = $def[ 0];
                        $old = new $objectAttrClass;
                    }
                    $this->_jsonAttrVals[ $name] = $old->set( $value);
                }
            } else {
                $this->_jsonAttrVals[ $name] = $value;
            }
        } else {
            $this->mapper->set( $name, $value);
        }
        return $value;
    }

    private function _setObjectAttrArr(
        $name, $oldObjArr, $newDataArr, $objectAttrClass, $count
    ) {
        if( null === $oldObjArr) {
            $oldObjArr = [];
        }
        if( null === $newDataArr) {
            $newDataArr = [];
        } else {
            $newDataArr = array_values( array_filter( $newDataArr));
        }
        $result = array_fill( 0, count( $newDataArr), null);
        // change and possibly move old objects
        // create new objects if there are none to change
        foreach( $result as $ix => &$resultObj) {
            foreach( $oldObjArr as &$oldObj) {
                if( $oldObj) {
                    $resultObj = $oldObj->set( $newDataArr[ $ix]);
                    $oldObj = null;
                    break;
            }   }
            if( ! $resultObj) {
                $resultObj = ( new $objectAttrClass)->set( $newDataArr[ $ix]);
        }   }
        // any remaining old are set() to NULL rather than simply discarded
        // see ObjectAttr->set() about "zombies"
        $this->_jsonAttrVals[ $name]
            = array_merge(
                $result,
                array_filter(
                    array_map(
                        function( $old) use( $objectAttrClass) {
                            return $old ? $old->set( null) : false;
                        },
                        $oldObjArr
            )   )   );
    }

    private function _cacheJsonAttr( $name, $def = null) {
        if( array_key_exists( $name, $this->_jsonAttrVals)) {
            return $this->_jsonAttrVals[ $name];
        }
        $dbData = $this->mapper->get( $name);
        if( ! $dbData) {
            return $this->_jsonAttrVals[ $name] = null;
        }
        $decoded = json_decode( $dbData, true);
        if( ! $decoded) {
            return $this->_jsonAttrVals[ $name] = null;
        }
        $def = is_array( $def) ? $def : $this->factory->jsonAttrs[ $name];
        if( ! $def) {
            return $this->_jsonAttrVals[ $name] = $decoded;
        }
        $attrClass = $def[ 0];
        if( count( $def) > 1) {
            return $this->_jsonAttrVals[ $name]
                    = array_map(
                        function( $objData) use( $attrClass) {
                            $objAttr = new $attrClass;
                            $objAttr->fromStore( $objData);
                            return $objAttr;
                        },
                        $decoded
                    );
        }
        $objAttr = new $attrClass;
        $objAttr->fromStore( $decoded);
        return $this->_jsonAttrVals[ $name] = $objAttr;
    }

    private function _assocFunc( $name) {
        return preg_match( "/(find|new|sorted)(.*)/", $name, $matches)
            && isset($this->factory->hasMany[$matches[2]])
            ? $matches : false;
    }
}

