use vicocomo::DatabaseIf;
pub fn test_single_pk(db: DatabaseIf) {
    use super::models::{find_or_insert_single_pk, single_pk::SinglePk};
    use vicocomo::{is_error, ActiveRecord, DbValue, QueryBld};

    super::models::reset_db(db.clone());

    println!("\nsimple primary key --------------------------------------\n");

    println!("inserting - - - - - - - - - - - - - - - - - - - - - - - -\n");

    let mut s = SinglePk {
        id: None,
        name: None,
        data: Some(17f32),
        opt: Some(2),
        un2: 1,
    };
    println!("inserting {:?} ..", s);
    assert!(s.insert(db.clone()).is_ok());
    assert!(s.id.is_some());
    let id = s.id.unwrap();
    assert_eq!(
        format!("{:?}", s),
        format!(
            "SinglePk {{ id: Some({}), name: Some(\"default\"), \
            data: Some(17.0), opt: Some(2), un2: 1 }}",
            id,
        ),
    );
    println!("    OK");
    let mut ss = vec![
        SinglePk {
            id: None,
            name: Some(String::from("hej")),
            data: None,
            opt: Some(1),
            un2: 1,
        },
        SinglePk {
            id: None,
            name: Some(String::from("hopp")),
            data: None,
            opt: Some(1),
            un2: 2,
        },
    ];
    println!("inserting batch {:?} ..", ss);
    let res = SinglePk::insert_batch(db.clone(), &mut ss[..]);
    assert!(res.is_ok());
    let res = res.unwrap();
    let id1 = res[0].id.unwrap();
    let id2 = res[1].id.unwrap();
    assert_eq!(
        format!("{:?}", res),
        format!(
            "[SinglePk {{ id: Some({}), name: Some(\"hej\"), data: None, \
            opt: Some(1), un2: 1 }}, \
            SinglePk {{ id: Some({}), name: Some(\"hopp\"), data: None, \
            opt: Some(1), un2: 2 }}]",
            id1, id2,
        ),
    );
    println!("    OK");

    println!("\nnot finding or updating non-existing  - - - - - - - - - -\n");

    let mut ne = SinglePk {
        id: Some(42000000),
        name: Some(String::from("hej")),
        data: None,
        opt: Some(1),
        un2: 42,
    };
    println!("not finding non-existing {:?} ..", ne);
    let res = ne.find_equal(db.clone());
    assert!(res.is_none());
    println!("    OK");
    println!("not finding non-existing by unique fields ..");
    assert!(SinglePk::find_by_name_and_un2(
        db.clone(),
        ne.name.as_ref().unwrap(),
        &ne.un2
    )
    .is_none());
    assert!(ne.find_equal_name_and_un2(db.clone()).is_none());
    println!("    OK");
    println!("error updating non-existing ..");
    let res = ne.update(db.clone());
    assert!(res.is_err());
    println!("    OK");
    println!("error updating to unique combination in another row ..");
    let ex = SinglePk::load(db.clone())
        .unwrap()
        .iter()
        .filter(|x| x.id != s.id)
        .next()
        .unwrap()
        .clone();
    s.name = ex.name.clone();
    s.un2 = ex.un2;
    let res = s.update(db.clone());
    assert!(res.is_err());
    assert!(is_error!(
        &res.err().unwrap(),
        Model(
            CannotSave,
            "SinglePk", Some("unique-violation".to_string()),
            "name", [],
            "un2", [],
        )
    ));
    println!("    OK");

    println!("\n,- transaction begin  - - - - - - - - - - - - - - - - - -");
    db.clone().begin().unwrap();
    println!("| inserting non-existing ..");
    let res = ne.insert(db.clone());
    assert!(res.is_ok());
    assert!(
        format!("{:?}", ne)
            == "SinglePk { id: Some(42000000), name: Some(\"hej\"), data: None, \
            opt: Some(1), un2: 42 }"
    );
    let mut un2 = 1000;
    let mut name = "aaa".to_string();
    for s in SinglePk::load(db.clone()).unwrap() {
        assert!(s.un2 <= un2);
        if s.un2 == un2 {
            assert!(s.name.clone().unwrap() >= name);
        }
        un2 = s.un2;
        name = s.name.unwrap().clone();
    }
    println!("|   OK");
    let mut s = SinglePk::find(db.clone(), &42000000).unwrap();
    s.name = Some("nytt namn".to_string());
    println!("| updating existing {:?} ..", s);
    let res = s.update(db.clone());
    assert!(res.is_ok());
    assert_eq!(
        format!("{:?}", s),
        "SinglePk { id: Some(42000000), name: Some(\"nytt namn\"), data: None, \
            opt: Some(1), un2: 42 }",
    );
    println!("|   OK");
    db.clone().commit().unwrap();
    println!("'- transaction commit - - - - - - - - - - - - - - - - - -");
    assert!(s.find_equal(db.clone()).is_some());
    println!("    OK");
    println!("error inserting existing ..");
    let mut ss = [s.clone(), s.clone(), s.clone()];
    ss[0].id = None;
    ss[0].un2 = 43;
    ss[2].id = None;
    let res = SinglePk::insert_batch(db.clone(), &mut ss);
    assert!(res.is_err());
    assert!(is_error!(
        &res.err().unwrap(),
        Model(
            CannotSave,
            "SinglePk", Some("unique-violation".to_string()),
            "id", [],
        )
    ));
    let res = s.insert(db.clone());
    assert!(res.is_err());
    println!("    OK");
    println!("error inserting existing unique combination ..");
    let mut su = s.clone();
    su.id = None;
    let res = su.insert(db.clone());
    assert!(res.is_err());
    assert!(is_error!(
        &res.err().unwrap(),
        Model(
            CannotSave,
            "SinglePk", Some("unique-violation".to_string()),
            "name", [],
            "un2", [],
        )
    ));
    let res = s.insert(db.clone());
    assert!(res.is_err());
    println!("    OK");

    println!("\nfinding existing  - - - - - - - - - - - - - - - - - - - -\n");

    println!("finding existing ..");
    let res = s.find_equal(db.clone());
    assert!(res.is_some());
    assert!(format!("{:?}", res.unwrap()) ==
        "SinglePk { id: Some(42000000), name: Some(\"nytt namn\"), data: None, \
            opt: Some(1), un2: 42 }"
    );
    println!("    OK");
    println!("finding existing by unique fields ..");
    let res =
        SinglePk::find_by_name_and_un2(db.clone(), s.name.as_ref().unwrap(), &s.un2);
    assert!(res.is_some());
    let res = res.unwrap();
    assert!(format!("{:?}", &res) ==
        "SinglePk { id: Some(42000000), name: Some(\"nytt namn\"), data: None, \
            opt: Some(1), un2: 42 }"
    );
    assert!(
        format!("{:?}", &res)
            == format!("{:?}", &s.find_equal_name_and_un2(db.clone()).unwrap())
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
    let found = SinglePk::query(db.clone(), &query);
    assert!(found.is_ok());
    let found = found.unwrap();
    let id1 = found[1].id.unwrap();
    let id2 = found[2].id.unwrap();
    assert_eq!(
        format!("{:?}", found),
        format!(
            "[\
                SinglePk {{ \
                    id: Some(42000000), \
                    name: Some(\"nytt namn\"), \
                    data: None, \
                    opt: Some(1), \
                    un2: 42 \
                }}, \
                SinglePk {{ \
                    id: Some({}), \
                    name: Some(\"default\"), \
                    data: Some(17.0), \
                    opt: Some(2), \
                    un2: 1 \
                }}, \
                SinglePk {{ \
                    id: Some({}), \
                    name: Some(\"hej\"), \
                    data: None, \
                    opt: Some(1), \
                    un2: 1 \
                }}\
            ]",
            id1, id2,
        ),
    );
    println!("    OK");
    println!("query() with custom order ..");
    let mut query = QueryBld::new()
        .col("un2")
        .eq(Some(&DbValue::Int(1)))
        .or("name")
        .gt(Some(&DbValue::NulText(Some(String::from("hopp")))))
        .order("opt, name DESC")
        .query()
        .unwrap();
    let found = SinglePk::query(db.clone(), &query);
    let found = found.unwrap();
    assert_eq!(
        format!("{:?}", found),
        format!(
            "[\
                SinglePk {{ \
                    id: Some(42000000), \
                    name: Some(\"nytt namn\"), \
                    data: None, \
                    opt: Some(1), \
                    un2: 42 \
                }}, \
                SinglePk {{ \
                    id: Some({}), \
                    name: Some(\"hej\"), \
                    data: None, \
                    opt: Some(1), \
                    un2: 1 \
                }}, \
                SinglePk {{ \
                    id: Some({}), \
                    name: Some(\"default\"), \
                    data: Some(17.0), \
                    opt: Some(2), \
                    un2: 1 \
                }}\
            ]",
            id2, id1,
        ),
    );
    println!("    OK");
    println!("query() without filter with custom order ..");
    let id3 = find_or_insert_single_pk(db.clone(), "hopp", 2).id.unwrap();
    query = QueryBld::new().order("opt, name DESC").query().unwrap();
    let found = SinglePk::query(db.clone(), &query);
    assert_eq!(
        format!("{:?}", found.unwrap()),
        format!(
            "[\
                SinglePk {{ \
                    id: Some(42000000), \
                    name: Some(\"nytt namn\"), \
                    data: None, \
                    opt: Some(1), \
                    un2: 42 \
                }}, \
                SinglePk {{ \
                    id: Some({}), \
                    name: Some(\"hopp\"), \
                    data: None, \
                    opt: Some(1), \
                    un2: 2 \
                }}, \
                SinglePk {{ \
                    id: Some({}), \
                    name: Some(\"hej\"), \
                    data: None, \
                    opt: Some(1), \
                    un2: 1 \
                }}, \
                SinglePk {{ \
                    id: Some({}), \
                    name: Some(\"default\"), \
                    data: Some(17.0), \
                    opt: Some(2), \
                    un2: 1 \
                }}\
            ]",
            id3, id2, id1,
        ),
    );
    println!("    OK");
    println!("query() without filter with limit ..");
    query.set_limit(Some(2));
    let found = SinglePk::query(db.clone(), &query);
    assert_eq!(
        format!("{:?}", found.unwrap()),
        format!(
            "[\
                SinglePk {{ \
                    id: Some(42000000), \
                    name: Some(\"nytt namn\"), \
                    data: None, \
                    opt: Some(1), \
                    un2: 42 \
                }}, \
                SinglePk {{ \
                    id: Some({}), \
                    name: Some(\"hopp\"), \
                    data: None, \
                    opt: Some(1), \
                    un2: 2 \
                }}\
            ]",
            id3,
        ),
    );
    println!("    OK");
    println!("query() without filter with offset ..");
    query.set_limit(None);
    query.set_offset(Some(1));
    let found = SinglePk::query(db.clone(), &query);
    assert_eq!(
        format!("{:?}", found.unwrap()),
        format!(
            "[\
                SinglePk {{ \
                    id: Some({}), \
                    name: Some(\"hopp\"), \
                    data: None, \
                    opt: Some(1), \
                    un2: 2 \
                }}, \
                SinglePk {{ \
                    id: Some({}), \
                    name: Some(\"hej\"), \
                    data: None, \
                    opt: Some(1), \
                    un2: 1 \
                }}, \
                SinglePk {{ \
                    id: Some({}), \
                    name: Some(\"default\"), \
                    data: Some(17.0), \
                    opt: Some(2), \
                    un2: 1 \
                }}\
            ]",
            id3, id2, id1,
        ),
    );
    println!("    OK");
    println!("query() without filter with limit and offset ..");
    query.set_limit(Some(2));
    query.set_offset(Some(1));
    let found = SinglePk::query(db.clone(), &query);
    assert_eq!(
        format!("{:?}", found.unwrap()),
        format!(
            "[\
                SinglePk {{ \
                    id: Some({}), \
                    name: Some(\"hopp\"), \
                    data: None, \
                    opt: Some(1), \
                    un2: 2 \
                }}, \
                SinglePk {{ \
                    id: Some({}), \
                    name: Some(\"hej\"), \
                    data: None, \
                    opt: Some(1), \
                    un2: 1 \
                }}\
            ]",
            id3, id2,
        ),
    );
    println!("    OK");

    println!(",- transaction begin  - - - - - - - - - - - - - - - - - -");
    db.clone().begin().unwrap();
    println!("| deleting existing {:?} ..", s);
    assert!(s.clone().delete(db.clone()).is_ok());
    println!("|   OK");
    println!("| error deleting non-existing {:?}", s);
    let res = s.clone().delete(db.clone());
    assert!(res.is_err());
    println!("|   OK");
    db.clone().rollback().unwrap();
    assert!(s.find_equal(db.clone()).is_some());
    println!("'- transaction rollback - - - - - - - - - - - - - - - - -");
    println!("    OK");

    println!("\nrequired => not zero or empty - - - - - - - - - - - - - -\n");

    let _ = db.clone().exec("DELETE FROM single_pks", &[]);

    println!("error inserting text with only whitespace ..");
    let mut s = SinglePk {
        id: Some(14654757),
        name: Some("\n \t ".to_string()),
        data: Some(17f32),
        opt: Some(2),
        un2: 1,
    };
    let res = s.insert(db.clone());
    assert!(is_error!(
        &res.err().unwrap(),
        Model(CannotSave, "SinglePk", None, "name", ["required"]),
    ));
    println!("    OK");
    println!("error inserting zero number ..");
    s.name = Some("a name".to_string());
    s.un2 = 0;
    let res = s.insert(db.clone());
    assert!(is_error!(
        &res.err().unwrap(),
        Model(CannotSave, "SinglePk", None, "un2", ["required"]),
    ));
    println!("    OK");
    println!("error updating text with only whitespace ..");
    s.un2 = 2;
    assert!(s.insert(db.clone()).is_ok());
    s.name = Some("\n \t ".to_string());
    let res = s.update(db.clone());
    assert!(is_error!(
        &res.err().unwrap(),
        Model(CannotSave, "SinglePk", None, "name", ["required"]),
    ));
    println!("    OK");
    println!("error updating zero number ..");
    s.name = Some("a name".to_string());
    s.un2 = 0;
    let res = s.update(db.clone());
    assert!(is_error!(
        &res.err().unwrap(),
        Model(CannotSave, "SinglePk", None, "un2", ["required"]),
    ));
    println!("    OK");

/*
    println!("\nvalidating  - - - - - - - - - - - - - - - - - - - - - - -\n");

    println!("presence ..");
    let mut s = SinglePk {
        id: None,
        name: None,
        data: Some(17f32),
        opt: Some(2),
        un2: 1,
    };
    assert!(s.validate_presence_of_data().is_ok());
    s.data = None;
    assert!(is_error!(s.validate_presence_of_data(), Model));
    println!("    OK");
*/
}
