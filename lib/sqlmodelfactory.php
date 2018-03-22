<?php

namespace Vicocomo;

/**
 * A factory that produces model objects using SqlModel.
 *
 * A major rationale for using the factory pattern is that it simplifies
 * initialization of instances of classes that use the SqlModel trait.
 */
class SqlModelFactory {
    use GeneralUtils;

    /**
     * READONLY!  The unqualified model class name, i.e. the last part of
     * $fullModelClassName.  For convenience this is a separate attribute.
     */
    public $modelName;

    /**
     * READONLY!  The name of the table in the database.
     */
    public $tableName;

    /**
     * READONLY!  The full class name, including namespace and starting with
     * "\", of the produced model objects.
     */
    public $fullModelClassName;

    /**
     * READONLY!  The database that the table lives in.
     */
    public $db;

    /**
     * READONLY!  Boolean, true iff $this->tableName is a view.
     */
    public $view;

    /**
     * READONLY!  An associative array where the keys are the names of the
     * columns in the database that are not part of the primary key.  If the
     * "fields" option to the constructor is given, only columns that are part
     * of the option value are included.
     *
     * The value is either <type> or a pair [ <type>, <limit>], where <type>
     * is the database type and <limit> is the maximum number of characters
     * that can be stored.
     */
    public $attrs = [];

    /**
     * READONLY!  For details see the "markdown" option to the constructor.
     */
    public $markdown = [];

    /**
     * READONLY!  An associative array [
     *   <attribute name> => [
     *       <optional qualified namspaced class name of ObjectAttr desc.>,
     *       <optional array size>
     *     ],
     *   ...
     * ]
     *
     * For details see the "json-attrs" and "object-attrs" options to the
     * constructor.
     */
    public $jsonAttrs = [];

    /**
     * READONLY!  An associative array where the values are assoc. arrays:
     * [ <has-many associaton name> => [
     *       "remote-model" => <remote modelName>,  // typically assoc name
     *       "foreign-key"  => <remote or join column referring to this>,
     *       "on-delete"    => <"cascade", "restrict", "set-null", NULL>,
     *       "through"      => <join modelName>,
     *       "remote-key"   => <join column referring to the remote model>,
     *     ], ...
     * ]
     *
     * If "through" is truthy "on-delete" is NULL and "remote-key" is set.  If
     * not, "remote-key" is not set.
     *
     * For details see the "has-many" option to the constructor.
     */
    public $hasMany = [];

    /**
     * READONLY!  An associative array [
     *   <name (with lowercase first character)> => [
     *       "remote-model" => <the remote factory modelName>,
     *       "foreign-key"  => <the foreign key name>
     *     ],
     *   ...
     * ]
     *
     * For details see the "belongs-to" option to the constructor.
     */
    public $belongsTo = [];

    /**
     * READONLY!  A function that compares the procuced objects.
     *
     * For details see the "compare" option to the constructor.
     */
    public $compare;

    /**
     * READONLY!  Handles to the framework and Vicocomo.
     */
    public $f3;
    public $vicocomo;

    /**
     * Construct a model factory.
     *
     *
     * $modelName and $namespc are used to set the property $modelName and
     * compute the property $fullModelClassName.
     *
     *
     * $options is an associative array for optional parameters as follows:
     *
     *
     * "table-name"
     *
     *   Optional table name in the database.  Default $modelName.
     *
     *
     * "db"
     *
     *   Optional database to use.  If present, the property $db is set to
     *   Base->setDb().  If not, Base->getDb().
     *
     *
     * "view"
     *
     *   If truthy, the database table should be a view and the option $view
     *   is set to true.  See SqlObject->delete() for why it is important to
     *   set this if the database table is actually a view.
     *
     *
     * "markdown"
     *
     *   Set the property "markdown".  It is an associative array where the
     *   keys are the names of the columns in the database that may contain
     *   markdown data.  This affects clean() and sanitize(); for markdown
     *   fields the tags in the F3 hive variable "vicocomoMarkdownTags" are
     *   not touched.
     *
     *
     * "json-attrs"
     *
     *   An attribute name or an array of them.  For each name, set
     *     $this->jsonAttrs[ <name>] = []
     *   This has the effect that the corresponding attribute is stored in the
     *   database as JSON.  When read from the database, JSON objects are
     *   decoded to PHP associative arrays.
     *
     *
     * "object-attrs"
     *
     *   Adds entries to the property "jsonAttrs" as follows.  It should be an
     *   associative array <name> => <def>, where <name> is an attribute that
     *   is stored in the database as JSON, see "json-attrs" above.
     *
     *   <def> is a positional array.  The first entry is the name of a class
     *   that handles getting and setting the data.  The class should use
     *   ObjectAttr.  The class name may but need not start with "\".  If it
     *   contains at least one "\", it is taken as a global name.  If not, the
     *   value of the F3 hive variable "vicocomo.modelNamespace" is prepended.
     *
     *   <def> may have a second element indicating the size of an array of
     *   ObjectAttr descendant instancess or 0 for variable size.  At present
     *   the value is ignored and only variable size arrays are handled.
     *
     *   The handling of objects is a collaboration between SqlModel,
     *   SqlModelFactory, and the class implementing ObjectAttr.
     *
     *
     * "has-many"
     *
     *   An array of associative arrays representing Factory instances
     *   associated to $this as follows:
     *
     *   "remote-name":
     *     Mandatory, used as the association name in the property $hasMany
     *     and to name the related functions in the property $assocFuncs as
     *     well as functions and properties in SqlObject.
     *
     *   "foreign-key":
     *     Optional name of the foreign key column in the remote table.  The
     *     default is fk<$this->modelName>.
     *
     *   "on-delete":
     *     Optional.  Ignored if "through", see below.  Should be one of
     *     "cascade", "restrict", or "set-null".  "restrict" is the default.
     *     See SqlObject->delete().
     *
     *   "remote-model":
     *     Optional remote model factory modelName.  Default is remote-name.
     *
     *   "through":
     *     If present
     *       - the association to "remote-name" is many to many,
     *       - a factory with this modelName is required to have a join table,
     *       - "foreign-key" refers to the join table,
     *       - "on-delete" is set to NULL.  It has no effect (the join table
     *         rows are always deleted if it is a base table and the remote
     *         table rows are never deleted).
     *
     *   "remote-key":
     *     Optional name of the foreign key column in the "through" join table
     *     referring to the remote model.  The default is fk<remote-model>.
     *
     *   Note that this allows for more than one has-many association from
     *   this model to *the same* remote model.
     *
     *   For each remote-name in has-many an entry in the property hasMany is
     *   created and SqlModel methods new<remote-name>() (not for "through"
     *   associations), find<remote-name>(), and sorted<remote-name>() will be
     *   available via PHP "overloading".
     *
     *
     * "belongs-to"
     *
     *   Array of associative arrays, each of which represents a factory that
     *   is referred to by a foreign key column in $this.  The array must
     *   contain the key "remote-name" which will be used to name the related
     *   property in \Models\Standard.  It may also contain:
     *
     *     "foreign-key":   The name of the foreign key column.  The default
     *                      is fk<$remote-name>.
     *
     *     "remote-model":  If present, it is the modelName referring to the
     *                      remote model factory.  The default is remote-name.
     *
     *     The first character of the name is converted to lowercase and
     *     becomes the key in the property belongsTo.  The value is an
     *     associcative array with keys "foreign-key" and "remote-model" and
     *     values as above.
     *
     *
     * "compare"
     *
     *   Defines the corresponding property as follows:
     *   - If falsy, the objects cannot be compared and the sorted() method
     *     will need a value for its second parameter.
     *   - If strictly true, the objects are compared using their compare()
     *     method.
     *   - If set to a string, the objects are compared on the value of the
     *     attribute with that name.  " DESC" may follow the attribute name to
     *     indicate reverse sorting.
     *   - If set to an array of strings (possibly ending in " DESC"), the
     *     comparision is on those attributes in that order.
     *   - If set to a comparision function, that is used.
     *
     *
     * "fields"
     *
     *   Used by modelInstance() as the fields parameter to the model object
     *   constructor.  The default is NULL.  If the value is truthy but "id"
     *   is not in it, "id" is added.
     *
     *
     * "schema-ttl"
     *
     *   Used by modelInstance() as the schemaTtl parameter to the model
     *   object constructor.  The default is 60.
     */
    function __construct( $modelName, $namespc, $options = []) {
        $vicocomo = Base::instance();
        $f3 = $vicocomo->f3;
        $this->vicocomo = $vicocomo;
        $this->f3 = $f3;
        $this->modelName = $modelName;
        $tableName = $this->getArrayVal( "table-name", $options, $modelName);
        $this->tableName = $tableName;
        $namespc = $this->getNamespace( "", $namespc);
        $this->fullModelClassName = "$namespc$modelName";
        $db = isset( $options[ "db"])
                ? $vicocomo->setDb( $options[ "db"]) : $vicocomo->getDb();
        $this->db = $db;
        $this->view = isset( $options[ "view"]) && $options[ "view"];
        $this->_schemaTtl = $this->getArrayVal( "schema-ttl", $options, 60);
        $fields = $this->getArrayVal( "fields", $options);
        if( is_string( $fields)) {
            $fields = $f3->split( $fields);
        }
        if( $fields && is_array( $fields) && ! in_array( "id", $fields)) {
            array_push( "id", $fields);
        }
        $this->_fields = $fields;
        $attrs = array_filter(
            array_map(
                function( $field) {
                    if( $field[ "pkey"]) {
                        return false;
                    }
                    $type = $field[ "type"];
                    if( preg_match(
                            '/(char|varchar)\((\d+)\)/', $type, $match
                        )
                    ) {
                        return [ $match[ 1],
                        (int) $match[ 2]];
                    }
                    return $type;
                },
                $db->schema( $tableName, $fields)
            )
        );
        $this->attrs = $attrs;
        $markdown = $this->getArrayVal( "markdown", $options);
        if( $markdown) {
            if( is_string( $markdown)) {
                $markdown = $f3->split( $markdown);
            }
            foreach( $markdown as $attr) {
                if( isset( $attrs[ $attr])) {
                    $this->markdown[ $attr] = true;
                }
            }
        }
        $jsonAttrs = [];
        foreach( $this->ensureArray(
                $this->getArrayVal( "json-attrs", $options, [])
            ) as $attr
        ) {
            $jsonAttrs[ $attr] = [];
        }
        foreach( $this->ensureArray(
                $this->getArrayVal( "object-attrs", $options, [])
            ) as $attr => $def
        ) {
            if( is_string( $def)) {
                $def = [ $def];
            }
            elseif( count( $def) > 1) {
                $def[ 1] = (int) $def[ 1];
            }
            $def[ 0] = $this->getNamespace(
                    $def[ 0], $f3->get( "vicocomo.modelNamespace")
                ) . $this->shortClassName( $def[ 0]);
            $jsonAttrs[ $attr] = $def;
        }
        $this->jsonAttrs = $jsonAttrs;
        $hasMany = [];
        foreach( $vicocomo->getArrayVal( "has-many", $options, []) as $assocDef
        ) {
            $name = $assocDef[ "remote-name"];
            unset( $assocDef[ "remote-name"]);
            if( ! isset( $assocDef[ "remote-model"])) {
                $assocDef[ "remote-model"] = $name;
            }
            if( ! isset( $assocDef[ "foreign-key"])) {
                $assocDef[ "foreign-key"] = "fk" . $modelName;
            }
            $through = $this->getArrayVal( "through", $assocDef);
            if( $through) {
                $assocDef[ "on-delete"] = NULL;
                if( ! isset( $assocDef[ "remote-key"])) {
                    $assocDef[ "remote-key"]
                        = "fk" . $assocDef[ "remote-model"];
                }
            } else {
                $onDel = $this->getArrayVal( "on-delete", $assocDef);
                $assocDef[ "on-delete"]
                    = in_array( $onDel, [ "cascade", "set-null"])
                      ? $onDel : "restrict";
                $assocDef[ "through"] = false;
                unset( $assocDef[ "remote-key"]);
            }
            $hasMany[ $name] = $assocDef;
        }
        $this->hasMany = $hasMany;
        $belongsTo = [];
        foreach( $this->getArrayVal( "belongs-to", $options, []) as $assocDef)
        {
            $name = $assocDef[ "remote-name"];
            unset( $assocDef[ "remote-name"]);
            if( ! isset( $assocDef[ "remote-model"])) {
                $assocDef[ "remote-model"] = $name;
            }
            if( ! isset( $assocDef[ "foreign-key"])) {
                $assocDef[ "foreign-key"] = "fk" . $name;
            }
            $belongsTo[ lcfirst( $name) ] = $assocDef;
        }
        $this->belongsTo = $belongsTo;
        $this->_getCompareAndOrderBy(
            $this->getArrayVal( "compare", $options)
        );
    }

    /**
     * Get the SqlModelFactory instance corresponding to $modelName, or NULL
     * if none found.
     */
    function factory( $modelName) {
        return $this->vicocomo->sqlModelFactory( $modelName);
    }

    /**
     * Get an array of modelName properties of all SqlModelFactory instances.
     */
    function sqlModels() {
        return $this->vicocomo->sqlModels();
    }

    /**
     * Count the rows in the table.
     */
    function count() {
        return 0 + reset(
                $this->db->exec( "SELECT COUNT( *) FROM $this->tableName;")
                    [ 0]
            );
    }

    function rowExists( $id) {
        return $id
            && $this->_newMapper()->findone( [ "id = :id", ":id" => $id]);
    }

    /**
     * Create a new model object.
     *
     * If $params is truthy but not an array the object is loaded from the
     * database using $params as the value of the "id" column.
     *
     * If $params is falsy or an array a new model object is created.
     */
    function modelInstance( $params = null) {
        if( $params && ! is_array( $params)) {
            $found = $this->find( [ "id = :id", ":id" => $params]);
            return $found ? $found[ 0] : null;
        } else {
            return new $this->fullModelClassName(
                    $this, $params, $this->_fields, $this->_schemaTtl
                );
        }
    }

    /**
     * modelInstance()->mapper->find(), but we want it to return instances of
     * the model class.
     *
     * If $options[ "model-class"] is a string the returned objects are
     * instances of that class instead, and that entry removed from $options.
     * The used model class must obviously be compatible with the table
     * structure of $this.
     *
     * After that, all args are forwarded to the mapper's find().
     */
    public function find( $filter = null, $options = [], $ttl = 0) {
        $usedCls = $this->_stripModelClassFrom( $options);
        return array_map(
            function( $mapper) use( $usedCls) {
                return new $usedCls( $this, $mapper, null, null);
            },
            $this->_newMapper()->find( $filter, $options, $ttl)
        );
    }

    /**
     * Adds a condition to the first argument to \DB\SQL\Mapper->find().
     * Returns the first argument with an added condition.
     *
     * $filter is an array of filter args for the find() function
     *   [<filter string, possibly with "?" placeholders>, value, ... ]
     *
     * $column and $value are added to the string and the array, respectively.
     *
     * $log is inserted before $column, and $rel between $column and a "?"
     * last in $filter[ 0].
     *
     * Handles a NULL or empty $filter.
     */
    function addFindCond( $filter, $column, $value, $rel = "=", $log = "AND"
    ) {
        $cond = $column . " " . $rel . " ?";
        if( $filter) {
            $result = gettype( $filter) === "string" ? [ $filter] : $filter;
            $result[ 0] .= " " . $log . " " . $cond;
        } else {
            $result = [ $cond];
        }
        $result[] = $value;
        return $result;
    }

    /**
     * find() and sort as indicated by the "compare" option to the
     * constructor.
     *
     * If the parameter $compare is set, it should be a callback (string or
     * array) sorting function to be used instead.
     *
     * See \DB\SQL\Mapper->find() about $filter, $options, and $ttl, but for
     * obvious reasons do not set $options[ "order"].
     *
     * Returns a sorted array.
     */
    function sorted( $filter = null, $options = [], $compare = null, $ttl = 0)
    {
        $callback = null;
        if( isset( $compare)) {
            $callback = $compare;
        } elseif( is_callable( $this->_orderBy)) {
            $callback = $this->_orderBy;
        }
        if( $callback) {
            $result = $this->find( $filter, $options, $ttl);
            usort( $result, $callback);
        } else {
            $result = $this->find( $filter,
                    array_merge( $options, [ "order" => $this->_orderBy]),
                    $ttl
                );
        }
        return $result;
    }

    /**
     * find() by the attribute-value pairs in the $attrVals array.
     *
     * If more than one object is found, return FALSE.
     *
     * If one object is found, return that.
     *
     * If no object is found and $ensure is falsy, return NULL.
     *
     * If no object is found and $ensure is truthy, try to create an object
     * with
     * modelInstance( <$attrVals, merged with $ensure if that is an array>).
     */
    function findUnique( $attrVals, $ensure = false, $ttl = 0) {
        if( ! $attrVals) {
            return null;
        }
        $filter = [];
        foreach( $attrVals as $attr => $value) {
            $filter = $this->addFindCond( $filter, $attr, $value);
        }
        $found = $this->find( $filter, [], $ttl);
        if( ! $found) {
            if( ! $ensure) {
                return null;
            }
            if( is_array( $ensure)) {
                $attrVals
                    = $this->sanitize( array_merge( $attrVals, $ensure));
            }
            return $this->modelInstance( $attrVals);
        }
        if( count( $found) == 1) {
            return $found[ 0];
        }
        return false;
    }

    /**
     * modelInstance()->mapper->findone(), but we want it to return an
     * instance of the model class.
     *
     * If $options[ "model-class"] is a string the returned object is an
     * instance of that class instead, and that entry removed from $options.
     * The used model class must obviously be compatible with the table
     * structure of $this.
     *
     * After that, all args are forwarded to the mapper's find().
     */
    public function findone( $filter = null, $options = [], $ttl = 0) {
        $usedCls = $this->_stripModelClassFrom( $options);
        return new $usedCls( $this,
                $this->_newMapper()->findone( $filter, $options, $ttl),
                null, null
            );
    }

    /**
     * Sort an array of model objects as indicated by the property compare.
     *
     * Returns a possibly sorted copy of $objArr.
     */
    function sort( $objArr) {
        $result = $objArr;
        if( ! is_callable( $this->compare)) {
            return $result;
        }
        usort( $result, $this->compare);
        return $result;
    }

    /**
     * Return a copy of $attrValue clean()ed by F3 except for members of the
     * markdown property we keep the tags listed in the hive variable
     * "vicocomoMarkdownTags".
     */
    function clean( $attrName, $attrValue) {
        return $this->f3->clean(
            $attrValue,
            isset( $this->markdown[ $attrName])
            ? $this->f3->get( "vicocomoMarkdownTags")
            : null
        );
    }

    /**
     * Return a copy of $params with
     * - entries with keys that are not column names removed.  If the option
     *   "fields" is given to the constructor only those are kept.
     * - the remaining values cleaned by clean().
     *
     * $options is an associtive array of options.  Currently recognized:
     *   "no-clean":  If truthy, do not run clean() on the values.
     *   "allow":     A positional array of extra attributes to keep.
     */
    function sanitize( $params, $options = null) {
      $result = [];
      $allowed = array_merge(
          array_keys( $this->attrs),
          $this->ensureArray( $this->getArrayVal( "allow", $options, []))
      );
      $clean = ! $this->getArrayVal( "no-clean", $options);
      foreach( $params as $key => $val) {
          if( in_array( $key, $allowed)) {
              $result[ $key] = $clean ? $this->clean( $key, $val) : $val;
          }
      }
      return $result;
    }

    /**
     * These are used by the SqlModel->{find,sorted}...() methods for has-many
     * associations.  May be useful if you have a database ID and want to find
     * associated objects without creating an instance.
     *
     * The first arg is the association name.
     *
     * The second arg is a flag to indicate sorting.  Any truthy value sorts
     * using the remote factory's sorted(), except "join-sort" which makes
     * findManyThrough() use the join factory's sorted() instead.
     *
     * The third arg is the ID of a row in $this table.
     *
     * The following args are forwarded to the remote factory's find() or
     * sorted() after adding a fiter tor return only records associated with
     * the second parameter.
     */
    function findMany() {
        $args   = func_get_args();
        $assNam = array_shift( $args);
        $func   = array_shift( $args) ? "sorted" : "find";
        $thisId = array_shift( $args);
        if( 0 === count( $args)) {
            $args[] = null;
        }
        $assoc = $this->hasMany[ $assNam];
        $args[ 0] = $this->addFindCond( $args[ 0], $assoc[ "foreign-key"],
                $thisId
            );
        return call_user_func_array( [
                    $this->vicocomo->sqlModelFactory(
                        $assoc[ "remote-model"]
                    ), $func
                ], $args
            );
    }
    function findManyThrough() {
        $args      = func_get_args();
        $assNam    = array_shift( $args);
        $sort      = array_shift( $args);
        $thisId    = array_shift( $args);
        $assoc     = $this->hasMany[ $assNam];
        $forKey    = $assoc[ "foreign-key"];
        $joinModel = $assoc[ "through"];
        $remKey    = $joinModel ? $assoc[ "remote-key"] : null;
        $remMod    = $assoc[ "remote-model"];
        if( "join-sort" == $sort) {
            $joinFunc = "sorted";
            $remFunc  = "find";
        } elseif( $sort) {
            $joinFunc = "find";
            $remFunc  = "sorted";
        } else {
            $joinFunc = "find";
            $remFunc  = "find";
        }
        $ids = array_map(
                function( $join) use( $remKey) {
                    return $join->$remKey;
                },
                $this->vicocomo->sqlModelFactory( $joinModel)->$joinFunc(
                    "$forKey = $thisId"
                )
            );
        if( ! $ids) {
            return [];
        }
        if( 0 == count( $args)) {
            $args[] = null;
        }
        $filter = reset( $args);
        if( $filter) {
            if( is_string( $filter)) {
                $filter =[ $filter];
            }
        } else {
            $filter = [ ""];
        }
        if( trim( $filter[ 0])) {
            $filter[ 0] .= " AND";
        }
        $filter[ 0] .= " id IN( " . implode( ", ", $ids) . ")";
        $args[ key( $args)] = $filter;
        $result = call_user_func_array(
               [ $this->vicocomo->sqlModelFactory( $remMod), $remFunc], $args
            );
        if( "join-sort" == $sort) {
            $idToIx = array_combine( array_values( $ids), array_keys( $ids));
            usort( $result,
                function( $o1, $o2) use( $idToIx) {
                    return $idToIx[ $o1->id] - $idToIx[ $o2->id];
                }
            );
        }
        return $result;
    }

    private $_orderBy = false;
    private $_fields;
    private $_schemaTtl;

    private function _getCompareAndOrderBy( $option) {
      if( ! $option) {
          $this->compare = $this->_orderBy = false;
          return;
      }
      if( $option === true) {
          $this->compare = $this->_orderBy =
              function( $o1, $o2) {
                  return $o1->compare( $o2);
              };
          return;
      }
      if( is_callable( $option)) {
          $this->compare = $this->_orderBy = $option;
          return;
      }
      if( is_string( $option)) {
          $this->_orderBy = $option;
          $option = [ $option];
      }
      $this->_orderBy = implode( ", ", $option);
      $attrs =
          array_map(
              function( $attrDesc) {
                  $attrDesc = array_filter( explode( " ", $attrDesc));
                  return[ reset( $attrDesc), "DESC" == next( $attrDesc)];
              },
              $option
          );
      $this->compare =
          function( $o1, $o2) use( $attrs) {
              foreach( $attrs as list( $attr, $desc)) {
                  if( $desc) {
                      list( $o1, $o2) = [ $o2, $o1];
                  }
                  if( $o1->$attr < $o2->$attr) {
                      return -1;
                  } elseif( $o1->$attr > $o2->$attr) {
                      return 1;
                  }
              }
              return 0;
          };
    }

    private function _newMapper() {
        return new \DB\SQL\Mapper(
                $this->db, $this->tableName, $this->_fields, $this->_schemaTtl
            );
    }

    private function _stripModelClassFrom( &$options) {
        if( isset( $options[ "model-class"])) {
            $result = $options[ "model-class"];
            unset( $options[ "model-class"]);
        } else {
            $result = $this->fullModelClassName;
        }
        return $result;
    }
}

