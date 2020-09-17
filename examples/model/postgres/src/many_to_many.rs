use super::models::{default_parent::DefaultParent, single_pk::SinglePk};
use ::vicocomo::{DbValue, QueryBld, Save};

pub fn test_many_to_many(db: &::vicocomo_postgres::PgConn) {

    /*let (m, _m2, _dp, bp, np) =*/ super::models::setup(db);
    //let s = single_pk(db, 1);

    println!("\nmany-to-many associations - - - - - - - - - - - - - - - -\n");

    println!("many-to-many ..");
    let mut pa = DefaultParent {
        id: None,
        name: "parent-a".to_string(),
    };
    pa.save(db).unwrap();
    let mut pb = DefaultParent {
        id: None,
        name: "parent-b".to_string(),
    };
    pb.save(db).unwrap();
    let mut sa = SinglePk {
        id: None,
        name: Some("child-a".to_string()),
        data: None,
        un1: None,
        un2: 101,
    };
    sa.save(db).unwrap();
    let mut sb = SinglePk {
        id: None,
        name: Some("child-b".to_string()),
        data: None,
        un1: None,
        un2: 102,
    };
    sb.save(db).unwrap();
    assert!(pa.connect_to_single_pk(db, &sa).is_ok());
    assert!(pa.connect_to_single_pk(db, &sa).is_err());
    assert!(pa.connect_to_single_pk(db, &sb).is_ok());
    assert!(pb.connect_to_single_pk(db, &sb).is_ok());
    assert!(pa.single_pks(db, None).unwrap().len() == 2);
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
    assert!(
        format!("{:?}", pb.disconnect_from_single_pk(db, &sa)) == "Ok(0)"
    );
    assert!(
        format!("{:?}", pb.disconnect_from_single_pk(db, &sb)) == "Ok(1)"
    );
    assert!(
        format!("{:?}", pa.disconnect_from_single_pk(db, &sa)) == "Ok(1)"
    );
    assert!(format!("{:?}", pa.single_pks(db, None)) == pa_sb_assoc);
    println!("    OK");
}
