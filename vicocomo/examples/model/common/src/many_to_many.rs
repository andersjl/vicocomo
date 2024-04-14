use super::models::reset_many_to_many;
use vicocomo::{is_error, DatabaseIf};

pub fn test_many_to_many(db: DatabaseIf) {
    use vicocomo::{DbValue, QueryBld};
    super::models::reset_db(db.clone());

    println!("\nmany-to-many associations -------------------------------\n");

    let (_dp, pa, pb, sa, sb) = reset_many_to_many(db.clone());
    println!("connect ..");
    assert!(pa.connect_to_single_pk(db.clone(), &sa).is_ok());
    assert!(pa.connect_to_single_pk(db.clone(), &sa).is_err());
    assert!(pa.connect_to_single_pk(db.clone(), &sb).is_ok());
    assert!(pb.connect_to_single_pk(db.clone(), &sb).is_ok());
    assert!(pa.single_pks(db.clone(), None).unwrap().len() == 2);
    println!("    OK");
    println!("find filtered ..");
    let pa_sb_assoc = format!(
        "Ok([SinglePk {{ id: Some({}), name: Some(\"child-b\"), data: None, \
        opt: Some(4711), un2: 102 }}])",
        sb.id.unwrap(),
    );
    assert_eq!(
        format!(
            "{:?}",
            pa.single_pks(
                db.clone(),
                QueryBld::new()
                    .col("name")
                    .eq(Some(&DbValue::Text("child-b".to_string())))
                    .query()
                    .as_ref(),
            ),
        ),
        pa_sb_assoc,
    );
    println!("    OK");
    println!("DB error connecting twice ..");
    assert!(pa.connect_to_single_pk(db.clone(), &sa).is_err());
    println!("    OK");
    println!("disconnecting not connected -> Ok(0) ..");
    assert!(
        format!("{:?}", pb.disconnect_from_single_pk(db.clone(), &sa))
            == "Ok(0)"
    );
    println!("    OK");
    println!("disconnecting connected -> Ok(1) ..");
    assert!(
        format!("{:?}", pb.disconnect_from_single_pk(db.clone(), &sb))
            == "Ok(1)"
    );
    assert!(
        format!("{:?}", pa.disconnect_from_single_pk(db.clone(), &sa))
            == "Ok(1)"
    );
    assert!(format!("{:?}", pa.single_pks(db.clone(), None)) == pa_sb_assoc);
    println!("    OK");

    println!("changing children -> Err(NYI) ..");
    let res = pa.save_single_pks(db.clone(), &[]);
    assert!(res.is_err());
    assert!(is_error!(&res.err().unwrap(), Other("NYI")));
    println!("    OK");
}
