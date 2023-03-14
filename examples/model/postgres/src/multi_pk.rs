use super::models::{
    find_or_insert_default_parent, multi_pk::MultiPk, multi_pk_templ,
};
use ::chrono::NaiveDate;
use ::vicocomo::{is_error, ActiveRecord, DatabaseIf, DbValue};

pub fn test_multi_pk(db: DatabaseIf) {
    let (m, m2, dp, _bp, _np) = super::models::reset_db(db);
    m.delete(db).unwrap(); // want to test insertion!
    m2.delete(db).unwrap();

    // --- MultiPk -----------------------------------------------------------
    println!("\ncomposite primary key -----------------------------------\n");

    let mut m = multi_pk_templ(&dp);

    // - - inserting, finding, and updating  - - - - - - - - - - - - - - - - -

    println!("inserting {} .. ", m.pk());
    assert!(m.insert(db).is_ok());
    assert_eq!(
        format!("{:?}", m),
        format!(
            "MultiPk {{ id: Some(1), id2: 1, bool_mand: false, \
            bool_mand_nul: None, f32_mand: 0.0, f32_opt: Some(1.0), \
            f64_mand: 0.0, f64_opt_nul: Some(Some(1.0)), i32_mand: 0, \
            i32_opt_nul: Some(Some(1)), default_parent_id: Some({}), \
            other_parent_id: None, bonus_parent: \"bonus nonstandard\", \
            date_mand: 0000-12-31, date_time_mand: 1970-01-01T00:00:00, \
            string_mand: \"\", u32_mand: 0, u64_mand: 0, usize_mand: 0 }}",
            dp.id.unwrap(),
        ),
    );
    println!("    OK");
    m.id2 = 42;
    println!("not finding non-existing {} ..", m.pk());
    assert!(MultiPk::find(db, &(42, 17)).is_none());
    assert!(m.find_equal(db).is_none());
    assert!(is_error!(
        MultiPk::validate_exists(db, &(m.id2, m.id.unwrap()), "message")
            .err()
            .unwrap(),
        Model(NotFound, "Self", Some("message".to_string())),
    ));
    assert!(m.validate_unique(db, "message").is_ok());
    println!("    OK");
    println!("error updating non-existing ..");
    let res = m.update(db);
    assert!(res.is_err());
    println!("    OK");
    println!("inserting non-existing ..");
    let res = m.insert(db);
    assert!(res.is_ok());
    println!("    OK");
    println!("finding existing ..");
    assert!(m == MultiPk::find(db, &(m.id.unwrap(), m.id2)).unwrap());
    assert!(m == m.find_equal(db).unwrap());
    assert!(
        MultiPk::validate_exists(db, &(m.id.unwrap(), m.id2), "message")
            .is_ok()
    );
    assert_eq!(
        m.validate_unique(db, "message").err().unwrap().to_string(),
        "error--Model-NotUnique\nerror--Model-NotUnique--Self--message",
    );
    println!("    OK");
    println!("error inserting existing ..");
    let res = m.insert(db);
    assert!(res.is_err());
    println!("    OK");
    println!("updating existing ..");
    let df = find_or_insert_default_parent(db, "default filler");
    m.bool_mand = true;
    m.bool_mand_nul = Some(false);
    m.f32_mand = 32.0;
    m.f32_opt = Some(32.0);
    m.f64_mand = 64.0;
    m.f64_opt_nul = Some(None);
    m.i32_mand = -32;
    m.i32_opt_nul = Some(Some(-32));
    m.default_parent_id = df.id;
    m.date_mand = NaiveDate::from_num_days_from_ce_opt(1).unwrap();
    m.string_mand = "hello".to_string();
    m.u32_mand = 32;
    m.u64_mand = 64;
    m.usize_mand = 1;
    m.update(db).unwrap();
    assert_eq!(
        format!("{:?}", m),
        format!(
            "MultiPk {{ id: Some(1), id2: 42, bool_mand: true, \
            bool_mand_nul: Some(false), f32_mand: 32.0, f32_opt: Some(32.0), \
            f64_mand: 64.0, f64_opt_nul: Some(None), i32_mand: -32, \
            i32_opt_nul: Some(Some(-32)), default_parent_id: Some({}), \
            other_parent_id: None, bonus_parent: \"bonus nonstandard\", \
            date_mand: 0001-01-01, date_time_mand: 1970-01-01T00:00:00, \
            string_mand: \"hello\", u32_mand: 32, u64_mand: 64, \
            usize_mand: 1 }}",
            df.id.unwrap(),
        ),
    );
    println!("    OK");
    println!("save() existing after change ..");
    m.usize_mand = 17;
    assert!(m.save(db).is_ok());
    println!("    OK");
    println!("finding existing ..");
    let res = m.find_equal(db);
    assert!(res.is_some());
    assert!(res.unwrap() == m);
    println!("    OK");
    println!("save() non-existing ..");
    let mut m2 = m.clone();
    m2.id2 = 17;
    m2.default_parent_id = df.id;
    assert!(m2.save(db).is_ok());
    assert!(m2.find_equal(db).unwrap() == m2);
    println!("update() invalid foreign key ..");
    m2.other_parent_id = Some("invalid".to_string());
    let res = m2.update(db);
    assert!(res.is_err());
    assert!(is_error!(
        &res.err().unwrap(),
        Model(
            CannotSave,
            "MultiPk", Some("foreign-key-violation".to_string()),
            "other_parent_id", [],
        )
    ));
    println!("    OK");
    println!("update_columns() invalid foreign key ..");
    m2.other_parent_id = m.other_parent_id.clone();
    let res = m2.update_columns(
        db,
        &[("other_parent_id", DbValue::Text("invalid".to_string()))],
    );
    assert!(res.is_err());
    assert!(res.err().unwrap().is_foreign_key_violation());
    println!("    OK");
    println!("insert() invalid foreign key ..");
    m2.id2 = 18;
    m2.other_parent_id = Some("invalid".to_string());
    let res = m2.insert(db);
    assert!(res.is_err());
    assert!(is_error!(
        &res.err().unwrap(),
        Model(
            CannotSave,
            "MultiPk", Some("foreign-key-violation".to_string()),
            "other_parent_id", [],
        )
    ));
    println!("    OK");
}
