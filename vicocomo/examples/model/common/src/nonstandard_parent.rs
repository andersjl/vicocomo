use vicocomo::DatabaseIf;
pub fn test_nonstandard_parent(db: DatabaseIf) {
    use super::models::other_parent::NonstandardParent;
    use vicocomo::{is_error, ActiveRecord};

    println!("\nmandatory primary key -----------------------------------\n");

    println!("inserting - - - - - - - - - - - - - - - - - - - - - - - -\n");

    let key = "the_key".to_string();
    println!("inserting new key ..");
    let mut n = NonstandardParent {
        pk: key.clone(),
        nonstandard_parent_id: None,
    };
    assert!(n.insert(db.clone()).is_ok());
    assert!(n.pk == key);
    println!("    OK");
    println!("error inserting old key ..");
    let res = n.insert(db.clone());
    assert!(res.is_err());
    assert!(is_error!(
        res.err().unwrap(),
        Model(
            CannotSave,
            "NonstandardParent",
            Some("unique-violation".to_string()),
            "pk",
            [],
        ),
    ));
    println!("    OK");
    println!("error inserting whitespace key because vicocomo_required ..");
    n.pk = " \t\n".to_string();
    let res = n.insert(db.clone());
    assert!(res.is_err());
    assert!(is_error!(
        res.err().unwrap(),
        Model(CannotSave, "NonstandardParent", None, "pk", ["required"],),
    ));
    println!("    OK");

    println!("error inserting batch with one invalid primary key ..");
    let mut nn = vec![
        n.clone(),
        NonstandardParent {
            pk: "valid".to_string(),
            nonstandard_parent_id: None,
        },
    ];
    let res = NonstandardParent::insert_batch(db.clone(), &mut nn[..]);
    assert!(res.is_err());
    assert!(is_error!(
        res.err().unwrap(),
        Model(CannotSave, "NonstandardParent", None, "pk", ["required"],),
    ));
    println!("    OK");
}
