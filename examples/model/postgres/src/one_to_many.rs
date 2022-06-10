use ::vicocomo::DatabaseIf;
pub fn test_one_to_many(db: DatabaseIf) {
    use super::models::{
        multi_pk::MultiPk, multi_pk_templ, other_parent::NonstandardParent,
    };
    use ::vicocomo::ActiveRecord;

    let (mut m, _m2, dp, bp, np) = super::models::reset_db(db);

    println!("\none-to-many associations --------------------------------\n");

    println!("finding children ..");
    let dp_chn = dp.multi_pks(db, None);
    assert!(dp_chn.is_ok());
    let dp_chn = dp_chn.unwrap();
    assert!(
        format!("{:?}", dp_chn)
            == format!("{:?}", m.default_parent_siblings(db).unwrap()),
    );
    let bp_chn = bp.bonus_childs(db, None);
    assert!(bp_chn.is_ok());
    let bp_chn = bp_chn.unwrap();
    assert!(
        format!("{:?}", bp_chn)
            == format!("{:?}", MultiPk::load(db).unwrap()),
    );
    let grown_chn = np.nonstandard_parents(db, None);
    assert!(grown_chn.is_ok());
    let grown_chn = grown_chn.unwrap();
    assert!(
        format!("{:?}", grown_chn)
            == format!(
                "{:?}",
                NonstandardParent::all_belonging_to_nonstandard_parent(
                    db, &np
                )
                .unwrap(),
            )
    );
    println!("    OK");

    println!("deleting children ..");
    assert!(dp.save_multi_pks(db, &[]).is_ok());
    assert!(dp.multi_pks(db, None).unwrap().is_empty());
    println!("    OK");

    println!("creating children ..");
    let mut mp1 = multi_pk_templ(&dp);
    mp1.id2 = 1;
    let mut mp2 = multi_pk_templ(&dp);
    mp2.id2 = 2;
    assert!(dp.save_multi_pks(db, &[mp1.clone(), mp2]).is_ok());
    let mps = dp.multi_pks(db, None);
    assert!(mps.is_ok());
    assert_eq!(mps.unwrap().len(), 2);
    println!("    OK");

    println!("changing children ..");
    let mut mp3 = multi_pk_templ(&dp);
    mp3.id2 = 3;
    assert!(dp.save_multi_pks(db, &[mp1, mp3]).is_ok());
    let mps = dp.multi_pks(db, None);
    assert!(mps.is_ok());
    assert_eq!(
        mps.unwrap().iter().map(|mp| mp.id2).collect::<Vec<_>>(),
        [1, 3],
    );
    println!("    OK");
}
