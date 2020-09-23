pub fn test_single_pk(db: &::vicocomo_postgres::PgConn) {
    use super::models::single_pk::SinglePk;
    use ::vicocomo::{DbConn, DbValue, Delete, Find, QueryBld, Save};

    super::models::setup(db);

    println!("\nsimple primary key --------------------------------------\n");

    println!("inserting - - - - - - - - - - - - - - - - - - - - - - - -\n");

    let mut s = SinglePk {
        id: None,
        name: None,
        data: Some(17f32),
        un1: Some(2),
        un2: 1,
    };
    println!("inserting {:?} ..", s);
    assert!(s.insert(db).is_ok());
    assert!(format!("{:?}", s) ==
        "SinglePk { id: Some(1), name: Some(\"default\"), data: Some(17.0), \
            un1: Some(2), un2: 1 }",
    );
    println!("    OK");
    let ss = vec![
        SinglePk {
            id: None,
            name: Some(String::from("hej")),
            data: None,
            un1: Some(1),
            un2: 1,
        },
        SinglePk {
            id: None,
            name: Some(String::from("hopp")),
            data: None,
            un1: Some(1),
            un2: 2,
        },
    ];
    println!("inserting batch {:?} ..", ss);
    let res = SinglePk::insert_batch(db, &ss[..]);
    assert!(res.is_ok());
    assert!(format!("{:?}", res) ==
        "Ok([SinglePk { id: Some(2), name: Some(\"hej\"), data: None, \
            un1: Some(1), un2: 1 }, \
            SinglePk { id: Some(3), name: Some(\"hopp\"), data: None, \
            un1: Some(1), un2: 2 }])"
    );
    println!("    OK");
    s = SinglePk {
        id: Some(42),
        name: Some(String::from("hej")),
        data: None,
        un1: Some(1),
        un2: 42,
    };

    println!("\nnot finding or updating non-existing  - - - - - - - - - -\n");

    println!("not finding non-existing {:?} ..", s);
    let res = s.find_equal(db);
    assert!(res.is_none());
    println!("    OK");
    println!("not finding non-existing by unique fields ..");
    assert!(
        SinglePk::find_by_un1_and_un2(db, s.un1.unwrap(), s.un2).is_none()
    );
    assert!(s.find_equal_un1_and_un2(db).is_none());
    assert!(
        SinglePk::validate_exists_un1_and_un2(
            db,
            s.un1.unwrap(),
            s.un2,
            "message"
        )
        .err()
        .unwrap()
        .to_string()
            == "Database error\nmessage: 1, 42"
    );
    assert!(s.validate_unique_un1_and_un2(db, "message").is_ok());
    println!("    OK");
    println!("error updating non-existing ..");
    let res = s.update(db);
    assert!(res.is_err());
    println!("    OK");

    println!("\n,- transaction begin  - - - - - - - - - - - - - - - - - -");
    db.begin().unwrap();
    println!("| inserting non-existing ..");
    let res = s.insert(db);
    assert!(res.is_ok());
    assert!(
        format!("{:?}", s)
            == "SinglePk { id: Some(42), name: Some(\"hej\"), data: None, \
            un1: Some(1), un2: 42 }"
    );
    let mut un2 = 1000;
    let mut name = "aaa".to_string();
    for s in SinglePk::load(db).unwrap() {
        assert!(s.un2 <= un2);
        if s.un2 == un2 {
            assert!(s.name.clone().unwrap() >= name);
        }
        un2 = s.un2;
        name = s.name.unwrap().clone();
    }
    println!("|   OK");
    s.name = Some("nytt namn".to_string());
    println!("| updating existing {:?} ..", s);
    let res = s.update(db);
    assert!(res.is_ok());
    assert!(format!("{:?}", s) ==
        "SinglePk { id: Some(42), name: Some(\"nytt namn\"), data: None, \
            un1: Some(1), un2: 42 }"
    );
    println!("|   OK");
    db.commit().unwrap();
    println!("'- transaction commit - - - - - - - - - - - - - - - - - -");
    assert!(s.find_equal(db).is_some());
    println!("    OK");
    println!("error inserting existing ..");
    let res = s.insert(db);
    assert!(res.is_err());
    println!("    OK");

    println!("\nfinding existing  - - - - - - - - - - - - - - - - - - - -\n");

    println!("finding existing ..");
    let res = s.find_equal(db);
    assert!(res.is_some());
    assert!(format!("{:?}", res.unwrap()) ==
        "SinglePk { id: Some(42), name: Some(\"nytt namn\"), data: None, \
            un1: Some(1), un2: 42 }"
    );
    println!("    OK");
    println!("finding existing by unique fields ..");
    let res = SinglePk::find_by_un1_and_un2(db, s.un1.unwrap(), s.un2);
    assert!(res.is_some());
    let res = res.unwrap();
    assert!(format!("{:?}", &res) ==
        "SinglePk { id: Some(42), name: Some(\"nytt namn\"), data: None, \
            un1: Some(1), un2: 42 }"
    );
    assert!(
        format!("{:?}", &res)
            == format!("{:?}", &s.find_equal_un1_and_un2(db).unwrap())
    );
    assert!(SinglePk::validate_exists_un1_and_un2(
        db,
        s.un1.unwrap(),
        s.un2,
        "message"
    )
    .is_ok());
    assert!(
        s.validate_unique_un1_and_un2(db, "message")
            .err()
            .unwrap()
            .to_string()
            == "Database error\nmessage: Some(1), 42"
    );
    println!("    OK");

    println!("\nquery() - - - - - - - - - - - - - - - - - - - - - - - - -\n");

    let query = QueryBld::new()
        .col("un2")
        .eq(Some(&DbValue::Int(1)))
        .or("name")
        .gt(Some(&DbValue::NulText(Some(String::from("hopp")))))
        .query()
        .unwrap();
    println!("query() with default order ..");
    let found = SinglePk::query(db, &query);
    assert!(
        format!("{:?}", found.unwrap())
            == "[\
            SinglePk { \
                id: Some(42), \
                name: Some(\"nytt namn\"), \
                data: None, \
                un1: Some(1), \
                un2: 42 \
            }, \
            SinglePk { \
                id: Some(1), \
                name: Some(\"default\"), \
                data: Some(17.0), \
                un1: Some(2), \
                un2: 1 \
            }, \
            SinglePk { \
                id: Some(2), \
                name: Some(\"hej\"), \
                data: None, \
                un1: Some(1), \
                un2: 1 \
            }\
        ]"
    );
    println!("    OK");
    println!("query() with custom order ..");
    let mut query = QueryBld::new()
        .col("un2")
        .eq(Some(&DbValue::Int(1)))
        .or("name")
        .gt(Some(&DbValue::NulText(Some(String::from("hopp")))))
        .order("un1, name DESC")
        .query()
        .unwrap();
    let found = SinglePk::query(db, &query);
    assert!(
        format!("{:?}", found.unwrap())
            == "[\
            SinglePk { \
                id: Some(42), \
                name: Some(\"nytt namn\"), \
                data: None, \
                un1: Some(1), \
                un2: 42 \
            }, \
            SinglePk { \
                id: Some(2), \
                name: Some(\"hej\"), \
                data: None, \
                un1: Some(1), \
                un2: 1 \
            }, \
            SinglePk { \
                id: Some(1), \
                name: Some(\"default\"), \
                data: Some(17.0), \
                un1: Some(2), \
                un2: 1 \
            }\
        ]"
    );
    println!("    OK");
    println!("query() without filter with custom order ..");
    query = QueryBld::new().order("un1, name DESC").query().unwrap();
    let found = SinglePk::query(db, &query);
    assert!(
        format!("{:?}", found.unwrap())
            == "[\
            SinglePk { \
                id: Some(42), \
                name: Some(\"nytt namn\"), \
                data: None, \
                un1: Some(1), \
                un2: 42 \
            }, \
            SinglePk { \
                id: Some(3), \
                name: Some(\"hopp\"), \
                data: None, \
                un1: Some(1), \
                un2: 2 \
            }, \
            SinglePk { \
                id: Some(2), \
                name: Some(\"hej\"), \
                data: None, \
                un1: Some(1), \
                un2: 1 \
            }, \
            SinglePk { \
                id: Some(1), \
                name: Some(\"default\"), \
                data: Some(17.0), \
                un1: Some(2), \
                un2: 1 \
            }\
        ]"
    );
    println!("    OK");
    println!("query() without filter with limit ..");
    query.set_limit(Some(2));
    let found = SinglePk::query(db, &query);
    assert!(
        format!("{:?}", found.unwrap())
            == "[\
            SinglePk { \
                id: Some(42), \
                name: Some(\"nytt namn\"), \
                data: None, \
                un1: Some(1), \
                un2: 42 \
            }, \
            SinglePk { \
                id: Some(3), \
                name: Some(\"hopp\"), \
                data: None, \
                un1: Some(1), \
                un2: 2 \
            }\
        ]"
    );
    println!("    OK");
    println!("query() without filter with offset ..");
    query.set_limit(None);
    query.set_offset(Some(1));
    let found = SinglePk::query(db, &query);
    assert!(
        format!("{:?}", found.unwrap())
            == "[\
            SinglePk { \
                id: Some(3), \
                name: Some(\"hopp\"), \
                data: None, \
                un1: Some(1), \
                un2: 2 \
            }, \
            SinglePk { \
                id: Some(2), \
                name: Some(\"hej\"), \
                data: None, \
                un1: Some(1), \
                un2: 1 \
            }, \
            SinglePk { \
                id: Some(1), \
                name: Some(\"default\"), \
                data: Some(17.0), \
                un1: Some(2), \
                un2: 1 \
            }\
        ]"
    );
    println!("    OK");
    println!("query() without filter with limit and offset ..");
    query.set_limit(Some(2));
    query.set_offset(Some(1));
    let found = SinglePk::query(db, &query);
    assert!(
        format!("{:?}", found.unwrap())
            == "[\
            SinglePk { \
                id: Some(3), \
                name: Some(\"hopp\"), \
                data: None, \
                un1: Some(1), \
                un2: 2 \
            }, \
            SinglePk { \
                id: Some(2), \
                name: Some(\"hej\"), \
                data: None, \
                un1: Some(1), \
                un2: 1 \
            }\
        ]"
    );
    println!("    OK");

    println!(",- transaction begin  - - - - - - - - - - - - - - - - - -");
    db.begin().unwrap();
    println!("| deleting existing {:?} ..", s);
    let res = s.clone().delete(db);
    assert!(res.is_ok());
    assert!(res.unwrap() == 1);
    println!("|   OK");
    println!("| error deleting non-existing {:?}", s);
    let res = s.clone().delete(db);
    assert!(res.is_err());
    println!("|   OK");
    db.rollback().unwrap();
    assert!(s.find_equal(db).is_some());
    println!("'- transaction rollback - - - - - - - - - - - - - - - - -");
    println!("    OK");
}
