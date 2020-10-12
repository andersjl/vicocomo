use super::models::{multi_pk::MultiPk, multi_pk_templ};
use ::chrono::NaiveDate;
use ::vicocomo::{Delete, Find, Save};

pub fn test_multi_pk(db: &::vicocomo_postgres::PgConn) {
    let (m, m2, _dp, _bp, _np) = super::models::setup(db);
    m.delete(db).unwrap(); // want to test insertion!
    m2.delete(db).unwrap();

    // --- MultiPk -----------------------------------------------------------
    println!("\ncomposite primary key -----------------------------------\n");

    let mut m = multi_pk_templ();

    // - - inserting, finding, and updating  - - - - - - - - - - - - - - - - -

    println!("inserting {} .. ", m.pk());
    assert!(m.insert(db).is_ok());
    assert!(
        format!("{:?}", m)
            == "MultiPk { id: Some(1), id2: 1, bool_mand: false, \
            bool_mand_nul: None, f32_mand: 0.0, f32_opt: Some(1.0), \
            f64_mand: 0.0, f64_opt_nul: Some(Some(1.0)), i32_mand: 0, \
            i32_opt_nul: Some(Some(1)), default_parent_id: 2, \
            other_parent_id: None, bonus_parent: \"bonus nonstandard\", \
            date_mand: 0000-12-31, date_time_mand: 1970-01-01T00:00:00, \
            string_mand: \"\", u32_mand: 0, u64_mand: 0, usize_mand: 0 }",
    );
    println!("    OK");
    m.id2 = 42;
    println!("not finding non-existing {} ..", m.pk());
    assert!(MultiPk::find(db, &(42, 17)).is_none());
    assert!(m.find_equal(db).is_none());
    assert!(
        MultiPk::validate_exists(db, &(m.id2, m.id.unwrap()), "message")
            .err()
            .unwrap()
            .to_string()
            == "Database error\nmessage"
    );
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
    assert!(
        m.validate_unique(db, "message").err().unwrap().to_string()
            == "Database error\nmessage"
    );
    println!("    OK");
    println!("error inserting existing ..");
    let res = m.insert(db);
    assert!(res.is_err());
    println!("    OK");
    println!("updating existing ..");
    m.bool_mand = true;
    m.bool_mand_nul = Some(false);
    m.f32_mand = 32.0;
    m.f32_opt = Some(32.0);
    m.f64_mand = 64.0;
    m.f64_opt_nul = Some(None);
    m.i32_mand = -32;
    m.i32_opt_nul = Some(Some(-32));
    m.default_parent_id = 1;
    m.date_mand = NaiveDate::from_num_days_from_ce(1);
    m.string_mand = "hello".to_string();
    m.u32_mand = 32;
    m.u64_mand = 64;
    m.usize_mand = 1;
    m.update(db).unwrap();
    assert!(
        format!("{:?}", m)
            == "MultiPk { id: Some(1), id2: 42, bool_mand: true, \
            bool_mand_nul: Some(false), f32_mand: 32.0, f32_opt: Some(32.0), \
            f64_mand: 64.0, f64_opt_nul: Some(None), i32_mand: -32, \
            i32_opt_nul: Some(Some(-32)), default_parent_id: 1, \
            other_parent_id: None, bonus_parent: \"bonus nonstandard\", \
            date_mand: 0001-01-01, date_time_mand: 1970-01-01T00:00:00, \
            string_mand: \"hello\", u32_mand: 32, u64_mand: 64, \
            usize_mand: 1 }",
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
    m2.default_parent_id = 1;
    assert!(m2.save(db).is_ok());
    assert!(m2.find_equal(db).unwrap() == m2);
    println!("    OK");
}
