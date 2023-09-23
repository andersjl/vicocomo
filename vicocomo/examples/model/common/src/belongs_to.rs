use super::models::{
    default_parent::DefaultParent, multi_pk::MultiPk,
    other_parent::NonstandardParent,
};
use vicocomo::{is_error, ActiveRecord, DatabaseIf};

pub fn test_belongs_to(db: DatabaseIf) {
    let (mut m, _m2, dp, mut bp, np) = super::models::reset_db(db.clone());
    let orig_dp_id = dp.id;

    println!("\nBelongsTo associations ----------------------------------\n");

    println!("setting saved parent ..");
    assert!(m.set_default_parent(&dp).is_ok());
    assert_eq!(m.default_parent_id, orig_dp_id);
    m.save(db.clone()).unwrap();
    assert!(m.set_nonstandard_parent(&np).is_ok());
    assert!(m.other_parent_id == Some("nonstandard".to_string()));
    assert!(m.set_bonus_parent(&bp).is_ok());
    assert!(m.bonus_parent == "bonus nonstandard");
    assert!(bp.set_nonstandard_parent(&np).is_ok());
    assert!(bp.nonstandard_parent_id == Some("nonstandard".to_string()));
    assert!(m.save(db.clone()).is_ok());
    assert!(bp.save(db.clone()).is_ok());
    println!("    OK");
    println!("forgetting optional parent ..");
    m.forget_nonstandard_parent();
    assert!(m.other_parent_id.is_none());
    assert!(m.save(db.clone()).is_ok());
    println!("    OK");
    println!("error setting parent w/o PK ..");
    let res = m.set_default_parent(&DefaultParent {
        id: None,
        name: "unsaved".to_string(),
    });
    assert!(res.is_err());
    assert!(is_error!(
        res.err().unwrap(),
        Model(
            Invalid,
            "MultiPk", None,
            "DefaultParent", ["missing-primary-key"],
        ),
    ));
    assert_eq!(m.default_parent_id, orig_dp_id);
    println!("    OK");
    println!("foreign key violation tested elsewhere for MultiPk");
    println!("getting saved parent ..");
    let dp = m.default_parent(db.clone());
    assert!(dp.is_some());
    let dp = dp.unwrap();
    assert_eq!(
        format!("{:?}", dp),
        format!(
            "DefaultParent {{ id: Some({}), name: \"used default\" }}",
            &orig_dp_id.unwrap(),
        ),
    );
    m.set_nonstandard_parent(&np)
        .and_then(|()| m.save(db.clone()))
        .unwrap();
    let np = m.nonstandard_parent(db.clone());
    assert!(np.is_some());
    let mut np = np.unwrap();
    assert_eq!(
        format!("{:?}", np),
        "NonstandardParent { \
            pk: \"nonstandard\", nonstandard_parent_id: None \
        }",
    );
    println!("    OK");
    println!("None w/o error if parent is not in DB");
    m.default_parent_id = Some(0);
    assert!(m.default_parent(db.clone()).is_none());
    m.default_parent_id = orig_dp_id;
    println!("    OK");
    println!("finding siblings ..");
    let dp_sibs = m.default_parent_siblings(db.clone());
    assert!(dp_sibs.is_ok());
    let dp_sibs = dp_sibs.unwrap();
    assert!(dp_sibs.len() == 2);
    assert_eq!(
        dp_sibs
            .iter()
            .filter(|s| s.default_parent_id == orig_dp_id)
            .count(),
        2,
    );
    let np_sibs: Result<Vec<MultiPk>, vicocomo::Error> =
        MultiPk::all_belonging_to_nonstandard_parent(db.clone(), &np);
    assert!(np_sibs.is_ok());
    let np_sibs = np_sibs.unwrap();
    assert!(np_sibs.len() == 1);
    let grown_sibs: Result<Vec<NonstandardParent>, vicocomo::Error> =
        NonstandardParent::all_belonging_to_nonstandard_parent(db.clone(), &np);
    assert!(grown_sibs.is_ok());
    let grown_sibs = grown_sibs.unwrap();
    assert!(grown_sibs.len() == 1);
    println!("    OK");
    println!("no siblings, no error if parent is not in DB ..");
    m.default_parent_id = Some(-4711);
    let dp_sibs = m.default_parent_siblings(db.clone());
    assert!(dp_sibs.is_ok());
    assert!(dp_sibs.unwrap().is_empty());
    np.pk = "unsaved".to_string();
    let np_sibs: Result<Vec<MultiPk>, vicocomo::Error> =
        MultiPk::all_belonging_to_nonstandard_parent(db.clone(), &np);
    assert!(np_sibs.is_ok());
    assert!(np_sibs.unwrap().is_empty());
    println!("    OK");
}
