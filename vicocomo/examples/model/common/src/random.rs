use vicocomo::DatabaseIf;
pub fn test_random(db: DatabaseIf) {
    use super::models::random::Random;
    use vicocomo::ActiveRecord;

    super::models::reset_db(db.clone());

    println!("\nvicocomo_random -----------------------------------------\n");

    let mut r = Random {
        id: None,
        data: None,
    };

    println!("inserting {:?} ..", r);
    let res = r.insert(db.clone());
    assert!(res.is_ok(), "{:?}", res);
    assert!(r.id.is_some());
    assert!(r.id.unwrap() > 1000); // poor man's test for randomness
    assert!(r.data.is_some());
    assert!(r.data.unwrap() > 1_000_000_000);
    println!("    OK");

    r.id = None;
    let data = r.data.unwrap();
    println!("inserting {:?} ..", r);
    let res = r.insert(db.clone());
    assert!(res.is_ok(), "{:?}", res);
    assert!(r.id.is_some());
    assert!(r.id.unwrap() > 1000);
    assert!(r.data == Some(data));
    println!("    OK");

    r.data = None;
    println!("updating {:?} ..", r);
    let res = r.update(db.clone());
    assert!(res.is_ok());
    assert!(r.data.is_some());
    assert!(r.data.unwrap() > 1_000_000_000);
    println!("    OK");

    r.data = Some(0);
    println!("updating {:?} ..", r);
    let res = r.update(db.clone());
    assert!(res.is_ok());
    assert!(r.data.is_some());
    assert!(r.data.unwrap() == 0);
    println!("    OK");
}
