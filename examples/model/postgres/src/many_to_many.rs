use super::models::setup_many_to_many;
use ::vicocomo::{DbValue, QueryBld};

pub fn test_many_to_many(db: &::vicocomo_postgres::PgConn) {

    /*let (m, _m2, _dp, bp, np) =*/ super::models::setup(db);
    //let s = single_pk(db, 1);

    println!("\nmany-to-many associations -------------------------------\n");

    let (pa, pb, sa, sb) = setup_many_to_many(db);
    println!("connect ..");
    assert!(pa.connect_to_single_pk(db, &sa).is_ok());
    assert!(pa.connect_to_single_pk(db, &sa).is_err());
    assert!(pa.connect_to_single_pk(db, &sb).is_ok());
    assert!(pb.connect_to_single_pk(db, &sb).is_ok());
    assert!(pa.single_pks(db, None).unwrap().len() == 2);
    println!("    OK");
    println!("find filtered ..");
    let pa_sb_assoc =
        "Ok([SinglePk { id: Some(2), name: Some(\"child-b\"), data: None, \
        un1: Some(4711), un2: 102 }])";
    assert!(
        format!(
            "{:?}",
            pa.single_pks(
                db,
                QueryBld::new()
                    .col("name")
                    .eq(Some(&DbValue::Text("child-b".to_string())))
                    .query()
                    .as_ref(),
            ),
        ) == pa_sb_assoc
    );
    println!("    OK");
    println!("DB error connecting twice ..");
    assert!(pa.connect_to_single_pk(db, &sa).is_err());
    println!("    OK");
    println!("disconnecting not connected -> Ok(0) ..");
    assert!(
        format!("{:?}", pb.disconnect_from_single_pk(db, &sa)) == "Ok(0)"
    );
    println!("    OK");
    println!("disconnecting connected -> Ok(1) ..");
    assert!(
        format!("{:?}", pb.disconnect_from_single_pk(db, &sb)) == "Ok(1)"
    );
    assert!(
        format!("{:?}", pa.disconnect_from_single_pk(db, &sa)) == "Ok(1)"
    );
    assert!(format!("{:?}", pa.single_pks(db, None)) == pa_sb_assoc);
    println!("    OK");
}
