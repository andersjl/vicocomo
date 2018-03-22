<?php

namespace Vicocomo;

/**
 * An interface that must be implemented by a class that handles a model
 * attribute that is stored as JSON in the database and requires more than
 * simply encoding and decoding.
 *
 * The JSON and the PHP value used when setting and getting are not
 * necessarily equivalent.  This is one reason for putting an object in
 * between.  Another reason is that it allows for handling of resources not
 * stored in the database, which is exactly what FileAttr does.
 *
 * The handling of the attribute is a collaboration between the class
 * implementing ObjectAttr, SqlModelFactory, and SqlModel.
 *
 * The database JSON may be data for one object or an array of them, the
 * attribute value as seen by PHP being single or an array.  See
 * SqlModelFactory and SqlModel for details on how this is handled.  But note
 * that the ObjectAttr interface only has methods that handle one
 * object/JSON/PHP value at a time.
 *
 *
 * "Zombies"
 *
 * If the object attribute instance handles resources that are not stored in
 * the database, it may be preferable not to create or destroy a resource when
 * set()-ting it, but rather postpone that to toStore().  This means that
 * set() may have to create a "zombie", i.e. an instance that looks like NULL
 * but still exists.  See set(), get(), and toStore().
 */
interface ObjectAttr {

    /**
     *
     * The class implementing ObjectAttr must have a constructor with no
     * mandatory parameters.
     */
    function __construct();

    /**
     * To initialize an attribute instance from the decoded JSON in the
     * database, SqlModel uses
     *   new <class implementing ObjectAttr>
     *   ->fromStore( <decoded JSON for one object>)
     * The return value is ignored.
     */
    function fromStore( $decodedData);

    /**
     * When getting a value, SqlModel returns
     *   <implementing class instance>->get()
     * which should return the PHP value.
     *
     * If the instance is a "zombie", see above, get() should return NULL.
     */
    function get();

    /**
     * When setting a value, SqlModel replaces the attribute instance with the
     * return value from
     *   <existing instance>->set( <PHP value>)
     * or, if there is no existing instance,
     *   new <class using ObjectAttr>->set( <PHP value>)
     *
     * If the object attribute is still valid after set, return $this or
     * possibly a new object.
     *
     * If no longer valid,, e.g. when $newPhpValue is NULL, the obvious thing
     * to return is NULL, which makes SqlModel forget the attribute instance.
     * An exception would be if the instance becomes a "zombie", see above.
     * Then set() should return $this.  Note that a subsequent get() should
     * return NULL.
     */
    function set( $newPhpValue);

    /**
     * Before saving to the database, SqlModel->collectErrorsPreventingStore()
     * calls the errorsPreventingStore() method in each attribute instance,
     * and adds any returned errors to the error list.
     */
    function errorsPreventingStore();

    /**
     * When saving the attribute to the database and before JSON encoding,
     * SqlModel replaces object attribute instances with the value of their
     * toStore() method.  toStore() should assume that errorsPreventingStore()
     * returned no errors.
     *
     * If the instance is a "zombie", see above, toStore() should destroy the
     * handled resource and return NULL.
     */
    function toStore();

    /**
     * Destroy any resources not stored in the database.
     *
     * Called by SqlModel before deleting the model object holding the
     * attribute from the database,
     */
    function delete();
}

