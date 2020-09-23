use super::models::{
    default_parent::DefaultParent,
    multi_pk::MultiPk,
    other_parent::NonstandardParent,
};
use ::vicocomo::{Find, Save};

pub fn test_belongs_to(db: &::vicocomo_postgres::PgConn) {
    let (mut m, _m2, _dp, mut bp, np) = super::models::setup(db);

    println!("\nBelongsTo associations ----------------------------------\n");

    println!("setting saved parent ..");
    assert!(m
        .set_default_parent(&DefaultParent::find(db, &2).unwrap(),)
        .is_ok(),);
    assert!(m.default_parent_id == 2);
    m.save(db).unwrap();
    assert!(m.set_nonstandard_parent(&np).is_ok());
    assert!(m.other_parent_id == Some("nonstandard".to_string()));
    assert!(m.set_bonus_parent(&bp).is_ok());
    assert!(m.bonus_parent == "bonus nonstandard");
    assert!(bp.set_nonstandard_parent(&np).is_ok());
    assert!(bp.nonstandard_parent_id == Some("nonstandard".to_string()));
    assert!(m.save(db).is_ok());
    assert!(bp.save(db).is_ok());
    println!("    OK");
    println!("unsetting parent ..");
    assert!(m.forget_nonstandard_parent().is_ok());
    assert!(m.other_parent_id.is_none());
    assert!(m.save(db).is_ok());
    println!("    OK");
    println!("error saving after setting parent w/o PK ..");
    assert!(m
        .set_default_parent(&DefaultParent {
            id: None,
            name: "unsaved".to_string(),
        })
        .is_err());
    assert!(m.default_parent_id == 2);
    println!("    OK");
    println!("error saving after setting parent with PK not in database ..");
    m.default_parent_id = 4711;
    assert!(m.save(db).is_err());
    m.default_parent_id = 2;
    assert!(m == MultiPk::find(db, &(1, 1)).unwrap());
    assert!(
        m.set_default_parent(
            &DefaultParent {
                id: Some(4711),
                name: "not saved".to_string(),
            },
        ).is_ok()
    );
    assert!(m.save(db).is_err());
    m.default_parent_id = 2;
    assert!(m == MultiPk::find(db, &(1, 1)).unwrap());
    println!("    OK");
    println!("getting saved parent ..");
    let dp = m.default_parent(db);
    assert!(dp.is_some());
    let dp = dp.unwrap();
    assert!(
        format!("{:?}", dp)
            == "DefaultParent { id: Some(2), name: \"used default\" }"
    );
    m.set_nonstandard_parent(&np)
        .and_then(|()| m.save(db))
        .unwrap();
    let np = m.nonstandard_parent(db);
    assert!(np.is_some());
    let np = np.unwrap();
    assert!(
        format!("{:?}", np)
            == "NonstandardParent { \
                pk: \"nonstandard\", nonstandard_parent_id: None \
            }"
    );
    println!("    OK");
    println!("finding siblings ..");
    let dp_sibs = m.default_parent_siblings(db);
    assert!(dp_sibs.is_ok());
    let dp_sibs = dp_sibs.unwrap();
    println!("{}", MultiPk::pks(&dp_sibs));
    assert!(dp_sibs.len() == 2);
    assert!(dp_sibs.iter().filter(|s| s.default_parent_id == 2).count() == 2);
    let np_sibs: Result<Vec<MultiPk>, ::vicocomo::Error> =
        MultiPk::all_belonging_to_nonstandard_parent(db, &np);
    assert!(np_sibs.is_ok());
    let np_sibs = np_sibs.unwrap();
    assert!(np_sibs.len() == 1);
    let grown_sibs: Result<Vec<NonstandardParent>, ::vicocomo::Error> =
        NonstandardParent::all_belonging_to_nonstandard_parent(db, &np);
    assert!(grown_sibs.is_ok());
    let grown_sibs = grown_sibs.unwrap();
    assert!(grown_sibs.len() == 1);
    println!("    OK");
}
