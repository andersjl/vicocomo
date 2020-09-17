use super::models::{multi_pk_templ, single_pk::SinglePk};
use ::vicocomo::{Delete, Save};

pub fn test_delete(db: &::vicocomo_postgres::PgConn) {

    let (m, _m2, _dp, bp, np) = super::models::setup(db);
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
    println!("error deleting non-existing {:?}", m.ids());
    let res = m.delete(db);
    assert!(res.is_err());
    println!("    OK");
    single_pk(db, 2);
    single_pk(db, 3);
    single_pk(db, 4);
    for (pks, del) in [([42, 43], 0), ([42, 3], 1), ([4, 2], 2)].iter() {
        println!("deleting {} out of batch {:?}", del, pks);
        let res = SinglePk::delete_batch(db, pks);
        assert!(res.is_ok());
        assert!(res.unwrap() == *del);
        println!("    OK");
    }
    println!("error deleting restricted parent ..");
    print!("    handled by database ..");
    let mut m17 = multi_pk_templ();
    m17.id2 = 17;
    m17.save(db).unwrap();
    m17.set_bonus_parent(&bp)
        .and_then(|()| m17.save(db))
        .unwrap();
    let old_counts = (
        np.multi_pks(db, None).unwrap().len(),
        np.nonstandard_parents(db, None).unwrap().len(),
    );
    let res = bp.clone().delete(db);
    assert!(res.is_err());
    let new_counts = (
        np.multi_pks(db, None).unwrap().len(),
        np.nonstandard_parents(db, None).unwrap().len(),
    );
    assert!(new_counts == old_counts);
    print!(" OK\n    handled by derive(Delete) ..");
/*
    m17.set_nonstandard_parent(&np)
        .and_then(|()| m17.save(db))
        .unwrap();
    let old_counts = (
        np.multi_pks(db, None).unwrap().len(),
        np.nonstandard_parents(db, None).unwrap().len(),
    );
    let res = np.clone().delete(db);
    assert!(res.is_err());
    let new_counts = (
        np.multi_pks(db, None).unwrap().len(),
        np.nonstandard_parents(db, None).unwrap().len(),
    );
    assert!(new_counts == old_counts);
    println!("    OK");
*/
    println!(" NOT TESTED\n    OK");
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

