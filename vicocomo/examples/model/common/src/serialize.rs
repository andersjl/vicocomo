use vicocomo::DatabaseIf;
pub fn test_serialize(db: DatabaseIf) {
    use super::models::serialize::{SerData, Serialize};
    use vicocomo::ActiveRecord;

    super::models::reset_db(db.clone());

    println!("\nvicocomo_serialize --------------------------------------\n");

    let mut s1 = Serialize {
        id: 1,
        not_null: SerData {
            pair: (1, 42),
            text: "not-null".to_string(),
        },
        nullable: None,
        optional: None,
        opt_null: None,
    };

    println!("inserting {s1:?} ..");
    let res = s1.insert(db.clone());
    assert!(res.is_ok(), "{:?}", res);
    assert_eq!(
        s1.optional,
        Some(SerData {
            pair: (0, 0),
            text: String::new()
        }),
    );
    assert_eq!(s1.opt_null, Some(None));
    assert_eq!(Serialize::find(db.clone(), &1), Some(s1.clone()));
    println!("    OK");

    println!("updating {s1:?} ..");
    s1.not_null.pair.1 = 43;
    s1.nullable = Some(SerData {
        pair: (1, 43),
        text: "nullable".to_string(),
    });
    s1.optional = Some(SerData {
        pair: (1, 43),
        text: "optional".to_string(),
    });
    s1.opt_null = Some(Some(SerData {
        pair: (1, 43),
        text: "optional".to_string(),
    }));
    assert!(s1.update(db.clone()).is_ok());
    assert_eq!(Serialize::find(db.clone(), &1), Some(s1.clone()));
    println!("    OK");

    let optional = SerData {
        pair: (2, 42),
        text: "optional".to_string(),
    };
    let mut s2 = Serialize {
        id: 2,
        not_null: SerData {
            pair: (2, 42),
            text: "not-null".to_string(),
        },
        nullable: None,
        optional: Some(optional.clone()),
        opt_null: None,
    };
    println!("inserting {s2:?} ..");
    let res = s2.insert(db.clone());
    assert!(res.is_ok(), "{:?}", res);
    assert_eq!(s2.optional, Some(optional.clone()));
    assert_eq!(Serialize::find(db.clone(), &2), Some(s2.clone()));
    println!("    OK");

    let opt_null = SerData {
        pair: (3, 42),
        text: "opt_null".to_string(),
    };
    let mut s3 = Serialize {
        id: 3,
        not_null: SerData {
            pair: (3, 42),
            text: "not-null".to_string(),
        },
        nullable: None,
        optional: None,
        opt_null: Some(Some(opt_null.clone())),
    };
    println!("inserting {s3:?} ..");
    let res = s3.insert(db.clone());
    assert!(res.is_ok(), "{:?}", res);
    assert_eq!(s3.opt_null, Some(Some(opt_null)));
    assert_eq!(Serialize::find(db.clone(), &3), Some(s3.clone()));
    println!("    OK");
}
