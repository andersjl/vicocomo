use super::models::{
    find_or_insert_single_pk, multi_pk::MultiPk, multi_pk_templ,
    other_parent::NonstandardParent, reset_many_to_many, single_pk::SinglePk,
};
use ::vicocomo::{is_error, ActiveRecord, DatabaseIf, DbValue};

pub fn test_delete(db: DatabaseIf) {
    let (m, _m2, dp, bp, np) = super::models::reset_db(db);
    let s = find_or_insert_single_pk(db, "", 1);

    println!("\ndeleting ------------------------------------------------\n");

    println!("deleting existing ..");
    assert!(s.clone().delete(db).is_ok());
    assert!(m.clone().delete(db).is_ok());
    println!("    OK");
    println!("error deleting struct w/o primary key");
    let mut no_prim = s.clone();
    no_prim.id = None;
    let res = no_prim.clone().delete(db);
    assert!(res.is_err());
    assert!(is_error!(
        &res.err().unwrap(),
        Model(
            CannotDelete,
            "SinglePk", Some("missing-primary-key".to_string()),
        ),
    ));
    println!("    OK");
    println!("error deleting non-existing MultiPk {:?}", m.pk());
    let res = m.delete(db);
    assert!(res.is_err());
    assert!(is_error!(
        &res.err().unwrap(),
        Model(
            CannotDelete,
            "MultiPk", Some("not-found".to_string()),
            "id", [],
            "id2", [],
        ),
    ));
    println!("    OK");
    println!("Error deleting if before_delete() returns Err");
    let mut s = find_or_insert_single_pk(db, "", 1);
    s.name = Some("immortal".to_string());
    let err = ::vicocomo::BeforeDelete::before_delete(&mut s, db);
    assert!(err.is_err());
    let err = err.err().unwrap();
    assert!(is_error!(
        err.clone(),
        Model(
            CannotDelete,
            "SinglePk", None,
            "name", ["Some(\"immortal\")"],
        ),
    ));
    let res = s.clone().delete(db);
    assert!(res.is_err());
    let res = res.err().unwrap();
    assert_eq!(res, err);
    println!("    OK");
    println!("OK deleting if before_delete() returns Ok");
    s.name = None;
    assert!(s.clone().delete(db).is_ok());
    println!("    OK");

    println!("delete_batch() empty slice");
    let res = SinglePk::delete_batch(db, &[]);
    assert!(res.is_ok());
    assert!(res.unwrap() == 0);
    println!("    OK");
    find_or_insert_single_pk(db, "", 2);
    find_or_insert_single_pk(db, "", 3);
    find_or_insert_single_pk(db, "", 4);
    for (pks, del) in [([42, 43], 0), ([42, 4], 1), ([5, 3], 2)].iter() {
        print!("deleting SinglePk, {} out of batch {:?} .. ", del, pks);
        let res = SinglePk::delete_batch(db, pks);
        if pks.len() == *del {
            assert!(res.is_ok());
            assert!(res.unwrap() == *del);
            println!("success");
        } else {
            assert!(is_error!(
                res.err().unwrap(),
                Model(
                    CannotDelete,
                    "SinglePk", Some("not-found".to_string()),
                    "id", [],
                ),
            ));
            println!("error (as expected!)");
        }
    }
    println!("    OK");
    println!("error deleting \"restrict\" parent ..");
    let mut m17 = multi_pk_templ(&dp);
    m17.id2 = 17;
    m17.save(db).unwrap();
    m17.set_bonus_parent(&bp)
        .and_then(|()| m17.save(db))
        .unwrap();
    let old_counts = (
        np.bonus_childs(db, None).unwrap().len(),
        np.multi_pks(db, None).unwrap().len(),
        np.nonstandard_parents(db, None).unwrap().len(),
    );
    let res = bp.clone().delete(db);
    assert!(res.is_err());
    assert!(is_error!(
        res.err().unwrap(),
        Model(
            CannotDelete,
            "NonstandardParent", Some("foreign-key-violation".to_string()),
            "BonusChild", [],
        ),
    ));
    let new_counts = (
        np.bonus_childs(db, None).unwrap().len(),
        np.multi_pks(db, None).unwrap().len(),
        np.nonstandard_parents(db, None).unwrap().len(),
    );
    assert!(new_counts == old_counts);
    let mut np17 = NonstandardParent {
        pk: 17.to_string(),
        nonstandard_parent_id: Some("nonstandard".to_string()),
    };
    np17.save(db).unwrap();
    let old_counts = (
        np.bonus_childs(db, None).unwrap().len(),
        np.multi_pks(db, None).unwrap().len(),
        np.nonstandard_parents(db, None).unwrap().len(),
    );
    let res = np.clone().delete(db);
    assert!(res.is_err());
    let new_counts = (
        np.bonus_childs(db, None).unwrap().len(),
        np.multi_pks(db, None).unwrap().len(),
        np.nonstandard_parents(db, None).unwrap().len(),
    );
    assert!(new_counts == old_counts);
    np17.clone().delete(db).unwrap();
    println!("    OK");
    println!("deleting \"forget\" parent ..");
    m17.set_bonus_parent(&bp).unwrap();
    m17.set_nonstandard_parent(&np)
        .and_then(|()| m17.save(db))
        .unwrap();
    assert!(np.clone().delete(db).is_ok());
    assert!(m17.nonstandard_parent(db).is_none());
    let m17 = MultiPk::find(db, &(m17.id.unwrap(), m17.id2)).unwrap();
    assert!(m17.other_parent_id == None);
    println!("    OK");
    println!("deleting \"cascade\" parent ..");
    assert!(dp.clone().delete(db).is_ok());
    assert!(m17.default_parent(db).is_none());
    assert!(MultiPk::find(db, &(m17.id.unwrap(), m17.id2)).is_none());
    println!("    OK");
    println!("deleting \"many-to-many\" parent ..");
    let (_dp, pa, pb, sa, sb) = reset_many_to_many(db);
    pa.connect_to_single_pk(db, &sa).unwrap();
    pa.connect_to_single_pk(db, &sb).unwrap();
    pb.connect_to_single_pk(db, &sa).unwrap();
    pb.connect_to_single_pk(db, &sb).unwrap();
    pa.clone().delete(db).unwrap();
    print!(" .. deletes connection rows ..");
    assert!(
        db.exec(
            "
            SELECT * FROM joins
                WHERE default_parent_id = $1 AND single_pk_id in ($2, $3)",
            &[
                DbValue::Int(pa.id.unwrap().into()),
                DbValue::Int(sa.id.unwrap().into()),
                DbValue::Int(sb.id.unwrap().into()),
            ],
        )
        .unwrap()
            == 0,
    );
    println!(" OK");
    print!(" .. leaves remote objects untouched ..");
    assert!(
        format!("{:?}", sa)
            == format!("{:?}", SinglePk::find(db, &sa.id.unwrap()).unwrap()),
    );
    assert!(
        format!("{:?}", sb)
            == format!("{:?}", SinglePk::find(db, &sb.id.unwrap()).unwrap()),
    );
    println!(" OK");
    println!("    OK");
    println!("deleting remote object ..");
    sa.clone().delete(db).unwrap();
    print!(" .. deletes connection rows ..");
    assert!(
        db.exec(
            "
            SELECT * FROM joins
                WHERE default_parent_id = $1 AND single_pk_id = $2",
            &[
                DbValue::Int(pb.id.unwrap().into()),
                DbValue::Int(sa.id.unwrap().into()),
            ],
        )
        .unwrap()
            == 0,
    );
    println!(" OK");
    println!("    OK");
}
