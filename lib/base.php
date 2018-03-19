<?php

namespace Vicocomo;

final class Base extends \Prefab {
    use GeneralUtils;

    /**
     * READONLY!  Shortcut to the Fat-Free Framework.
     */
    public $f3;

    /**
     * READONLY!  The integer Unix timestamp when the framwork was
     * initialized.
     */
    public $now;

    /**
     * Initialize the singleton.
     */
    function __construct() {
        $this->f3 = \Base::instance();
        $this->setLanguage( null);
        $this->now = (int) $this->f3->get( "TIME");
    }

    /**
     *  Set the language used by $this->strcmp().  See Collator.  In lieu of
     *  setLanguage() the default locale is used.
     */
    function setLanguage( $lang) {
        $this->_collator = new \Collator( $lang);
    }

//--- SqlModel utilities -----------------------------------------------------

    /**
     * Get the SqlModelFactory with modelName name.
     */
    function sqlModelFactory( $name) {
        return $this->getArrayVal( $name, $this->_sqlModelFactories);
    }

    /**
     * Get the modelName-s of all registered SqlModelFactory instances.
     */
    function sqlModels() {
        return array_keys( $this->_sqlModelFactories);
    }

    /**
     * Create one or more SqlModelFactory instances from the model class
     * static methods factoryOptions().
     *
     * $db is a \DB\SQL instance.  It is sent to setDb(), see that.
     *
     * $models is one model class name or a positional array of them.  A class
     * name may but need not start with "\".  If it contains at least one "\",
     * it is taken as a global name.  If not, the value of the F3 hive
     * variable "vicocomo.modelNamespace" is prepended.
     *
     * For each model class the options to the SqlModelFactory constructor are
     * read from <model class>::configure().  The parameters $namespc and
     * $modelName are extracted from the model class, which means that the
     * unqualified class name becomes the $modelName property of the factory
     * and is returned by sqlModelFactories() and used to retrieve the factory
     * by sqlModelFactory().  So you may have model classes with more than one
     * namespace, but each unqualified class name must still be unique.
     *
     * If the table exists, the factory is created.  If not, no more factories
     * are created.
     *
     * Returns an array of created modelName-s on success or falsy on failure.
     */
    function createSqlModelFactories( $models, $db = null) {
        $models = $this->ensureArray( $models);
        if( $db) {
            $this->setDb( $db);
        }
        $db = $this->getDb();
        if( ! $db) {
            return false;
        }
        $created = [];
        $success = true;
        foreach( $models as $model) {
          $modelName     = $this->shortClassName( $model);
          $namespc = $this->getNamespace( $model,
                          $this->f3->get( "vicocomo.modelNamespace")
                      );
          $modelClass    = "$namespc$modelName";
          $options       = $modelClass::factoryOptions() ? : [];
          $options["db"] = $db;
          try {
              $factory = new SqlModelFactory( $modelName, $namespc, $options);
          }
          catch( \Exception $e) {
              return false;
          }
          $created[$modelName] = $factory;
          $this->_sqlModelFactories[$modelName] = $factory;
        }
        return array_keys( $created);
    }

//--- Database utilities -----------------------------------------------------

    /**
     * Set the current database.
     *
     * $db is a \DB\SQL instance.  It is remembered, and the next call to
     * getDb() will return it.
     *
     * If called with a $db that is equal (==) to one previously used, the
     * older $db is left in place and returned.  Otherwise $db is reurned.
     */
    function setDb( $db) {
        $oldDbKey = array_search( $db, $this->_databases);
        if( false === $oldDbKey) {
            array_unshift( $this->_databases, $db);
            $this->_curDbKey = 0;
        } else {
            $this->_curDbKey = $oldDbKey;
        }
        return $this->_databases[$this->_curDbKey];
    }

    /**
     * Return the current database or NULL if no database known.
     *
     * Before the first call to setDb() a call to getDb() will imply
     * setDb( <the F3 hive variable "vicocomo.database">).
     */
    function getDb() {
      if( ! isset( $this->_curDbKey)) {
          if( $this->f3->exists( "vicocomo.database", $db)) {
              return $this->setDb( $db);
          } else {
              return null;
          }
      }
      return $this->_databases[$this->_curDbKey];
    }

//--- Private section --------------------------------------------------------

    private $_collator;
    private $_curDbKey;
    private $_databases = [];
    private $_sqlModelFactories = [];
}

return Base::instance();

