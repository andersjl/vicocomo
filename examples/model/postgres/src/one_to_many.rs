pub fn test_one_to_many(db: &::vicocomo_postgres::PgConn) {
    use super::models::{multi_pk::MultiPk, other_parent::NonstandardParent};
    use ::vicocomo::Find;

    let (mut m, _m2, dp, bp, np) = super::models::setup(db);

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
}
