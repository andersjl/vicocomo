use super::models::{
    multi_pk::MultiPk, multi_pk_templ, other_parent::NonstandardParent,
    setup_many_to_many, single_pk::SinglePk,
};
use ::vicocomo::{DbConn, DbValue, Delete, Find, Save};

pub fn test_delete(db: &::vicocomo_postgres::PgConn) {

    let (m, _m2, dp, bp, np) = super::models::setup(db);
    let s = single_pk(db, 1);

    println!("\ndeleting ------------------------------------------------\n");

    println!("deleting existing ..");
    let res = s.clone().delete(db);
    assert!(res.is_ok());
    assert!(res.unwrap() == 1);
    let res = m.clone().delete(db);
    assert!(res.is_ok());
    assert!(res.unwrap() == 1);
    println!("    OK");
    println!("error deleting non-existing {:?}", m.pk());
    let res = m.delete(db);
    assert!(res.is_err());
    println!("    OK");
    single_pk(db, 2);
    single_pk(db, 3);
    single_pk(db, 4);
    for (pks, del) in [([42, 43], 0), ([42, 3], 1), ([4, 2], 2)].iter() {
        print!("deleting {} out of batch {:?} .. ", del, pks);
        let res = SinglePk::delete_batch(db, pks);
        if pks.len() == *del {
            assert!(res.is_ok());
            assert!(res.unwrap() == *del);
            println!("success");
        } else {
            assert!(res.is_err());
            println!("error (as expected!)");
        }
    }
    println!("    OK");
    println!("error deleting \"restrict\" parent ..");
    let mut m17 = multi_pk_templ();
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
    let (pa, pb, sa, sb) = setup_many_to_many(db);
    pa.connect_to_single_pk(db, &sa).unwrap();
    pa.connect_to_single_pk(db, &sb).unwrap();
    pb.connect_to_single_pk(db, &sa).unwrap();
    pb.connect_to_single_pk(db, &sb).unwrap();
    pa.clone().delete(db).unwrap();
    print!(" .. deletes connection rows ..");
    assert!(
        db.exec("
            SELECT * FROM joins
                WHERE default_parent_id = $1 AND single_pk_id in ($2, $3)",
            &[
                DbValue::Int(pa.id.unwrap().into()),
                DbValue::Int(sa.id.unwrap().into()),
                DbValue::Int(sb.id.unwrap().into()),
            ],
        ).unwrap() == 0,
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
        db.exec("
            SELECT * FROM joins
                WHERE default_parent_id = $1 AND single_pk_id = $2",
            &[
                DbValue::Int(pb.id.unwrap().into()),
                DbValue::Int(sa.id.unwrap().into()),
            ],
        ).unwrap() == 0,
    );
    println!(" OK");
    println!("    OK");
}

fn single_pk(db: &::vicocomo_postgres::PgConn, un2: i32) -> SinglePk {
    let mut result = SinglePk {
        id: None,
        name: None,
        data: None,
        un1: None,
        un2: un2,
    };
    result.save(db).unwrap();
    result
}

