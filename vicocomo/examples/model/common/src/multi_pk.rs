use super::models::{
    find_or_insert_default_parent, multi_pk::MultiPk, multi_pk_templ,
    random::Random,
};
use chrono::NaiveDate;
use vicocomo::{is_error, ActiveRecord, DatabaseIf, DbValue};

pub fn test_multi_pk(db: DatabaseIf) {
    let (m, m2, dp, _bp, _np) = super::models::reset_db(db.clone());
    m.delete(db.clone()).unwrap(); // want to test insertion!
    m2.delete(db.clone()).unwrap();

    // --- MultiPk -----------------------------------------------------------
    println!("\ncomposite primary key -----------------------------------\n");

    let mut m = multi_pk_templ(&dp);

    // - - inserting, finding, and updating  - - - - - - - - - - - - - - - - -

    println!("inserting {} .. ", m.pk());
    assert!(m.insert(db.clone()).is_ok());
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
    assert!(MultiPk::find(db.clone(), &(42, 17)).is_none());
    assert!(m.find_equal(db.clone()).is_none());
    assert!(is_error!(
        MultiPk::validate_exists(
            db.clone(),
            &(m.id2, m.id.unwrap()),
            "message"
        )
        .err()
        .unwrap(),
        Model(NotFound, "Self", Some("message".to_string())),
    ));
    assert!(m.validate_unique(db.clone(), "message").is_ok());
    println!("    OK");
    println!("error updating non-existing ..");
    let res = m.update(db.clone());
    assert!(res.is_err());
    println!("    OK");
    println!("inserting non-existing ..");
    let res = m.insert(db.clone());
    assert!(res.is_ok());
    println!("    OK");
    println!("finding existing ..");
    assert!(m == MultiPk::find(db.clone(), &(m.id.unwrap(), m.id2)).unwrap());
    assert!(m == m.find_equal(db.clone()).unwrap());
    assert!(MultiPk::validate_exists(
        db.clone(),
        &(m.id.unwrap(), m.id2),
        "message"
    )
    .is_ok());
    assert_eq!(
        m.validate_unique(db.clone(), "message")
            .err()
            .unwrap()
            .to_string(),
        "error--Model-NotUnique\nerror--Model-NotUnique--Self--message",
    );
    println!("    OK");
    println!("error inserting existing ..");
    let res = m.insert(db.clone());
    assert!(res.is_err());
    println!("    OK");
    println!("updating existing ..");
    let df = find_or_insert_default_parent(db.clone(), "default filler");
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
    m.string_mand = "hello\n',world".to_string();
    m.u32_mand = 32;
    m.u64_mand = 64;
    m.usize_mand = 1;
    m.update(db.clone()).unwrap();
    assert_eq!(
        format!("{:?}", m),
        format!(
            "MultiPk {{ id: Some(1), id2: 42, bool_mand: true, \
            bool_mand_nul: Some(false), f32_mand: 32.0, f32_opt: Some(32.0), \
            f64_mand: 64.0, f64_opt_nul: Some(None), i32_mand: -32, \
            i32_opt_nul: Some(Some(-32)), default_parent_id: Some({}), \
            other_parent_id: None, bonus_parent: \"bonus nonstandard\", \
            date_mand: 0001-01-01, date_time_mand: 1970-01-01T00:00:00, \
            string_mand: \"hello\\n',world\", u32_mand: 32, u64_mand: 64, \
            usize_mand: 1 }}",
            df.id.unwrap(),
        ),
    );
    println!("    OK");
    println!("save() existing after change ..");
    m.usize_mand = 17;
    assert!(m.save(db.clone()).is_ok());
    println!("    OK");
    println!("finding existing ..");
    let res = m.find_equal(db.clone());
    assert!(res.is_some());
    assert!(res.unwrap() == m);
    println!("    OK");
    println!("save() non-existing ..");
    let mut m2 = m.clone();
    m2.id2 = 17;
    m2.default_parent_id = df.id;
    assert!(m2.save(db.clone()).is_ok());
    assert!(m2.find_equal(db.clone()).unwrap() == m2);
    println!("update() invalid foreign key ..");
    m2.other_parent_id = Some("invalid".to_string());
    let res = m2.update(db.clone());
    assert!(res.is_err());
    assert!(is_error!(
        &res.err().unwrap(),
        Model(
            CannotSave,
            "MultiPk",
            Some("foreign-key-violation".to_string()),
            "other_parent_id",
            [],
        )
    ));
    println!("    OK");
    println!("update_columns() invalid foreign key ..");
    m2.other_parent_id = m.other_parent_id.clone();
    let res = m2.update_columns(
        db.clone(),
        &[("other_parent_id", DbValue::Text("invalid".to_string()))],
    );
    assert!(res.is_err());
    assert!(res.err().unwrap().is_foreign_key_violation());
    println!("    OK");
    println!("insert() invalid foreign key ..");
    m2.id2 = 18;
    m2.other_parent_id = Some("invalid".to_string());
    let res = m2.insert(db.clone());
    assert!(res.is_err());
    assert!(is_error!(
        &res.err().unwrap(),
        Model(
            CannotSave,
            "MultiPk",
            Some("foreign-key-violation".to_string()),
            "other_parent_id",
            [],
        )
    ));
    println!("    OK");

    // - - inserting, finding, and updating  - - - - - - - - - - - - - - - - -

    println!("\nbackup and restore --------------------------------------\n");
    {
        // keeping some data local to these tests
        println!("try_to_sql() with data in table ..");
        let sql = MultiPk::try_to_sql(db.clone());
        assert!(sql.is_ok());
        let sql = sql.unwrap();
        assert_eq!(
            sql,
            "INSERT INTO multi_pks (\
                id, id2, bool_mand, bool_mand_nul, \
                f32_mand, f32_opt, f64_mand, f64_opt_nul, \
                i32_mand, i32_opt_nul, default_parent_id, other_parent_id, \
                bonus_parent, date_mand, date_time_mand, string_mand, \
                u32_mand, u64_mand, usize_mand\
            ) VALUES (\
                1, 1, 0, NULL, \
                0, 1, 0, 1, \
                0, 1, 18, NULL, \
                'bonus nonstandard', 0, 0, '', \
                0, 0, 0\
            ), (\
                1, 42, 1, 0, \
                32, 32, 64, NULL, \
                -32, -32, 17, NULL, \
                'bonus nonstandard', 1, 0, 'hello\n'',world', \
                32, 64, 17\
            ), (\
                1, 17, 1, 0, \
                32, 32, 64, NULL, \
                -32, -32, 17, NULL, \
                'bonus nonstandard', 1, 0, 'hello\n'',world', \
                32, 64, 17\
            );",
        );
        println!("    OK");
        println!("try_to_sql() with empty table ..");
        let rnd_sql = Random::try_to_sql(db.clone());
        assert!(rnd_sql.is_ok());
        assert!(rnd_sql.unwrap().is_empty());
        println!("    OK");

        println!("try_sql_to_csv() from --- table name --- ..");
        let csv = Random::try_sql_to_csv("", Some(b';'), true);
        assert!(csv.is_ok());
        let (table, csv) = csv.unwrap();
        assert_eq!(table, "randoms");
        assert_eq!(csv, "\r\n");
        println!("    OK");
        println!("try_sql_to_csv() form INERT SQK ..");
        let csv = MultiPk::try_sql_to_csv(&sql, Some(b';'), true);
        assert!(csv.is_ok());
        let (table, csv) = csv.unwrap();
        assert_eq!(table, "multi_pks");
        assert_eq!(
            csv,
            "id;id2;bool_mand;bool_mand_nul;f32_mand;f32_opt;f64_mand;\
                f64_opt_nul;i32_mand;i32_opt_nul;default_parent_id;\
                other_parent_id;bonus_parent;date_mand;date_time_mand;\
                string_mand;u32_mand;u64_mand;usize_mand\r\n\
                1;1;0;;0;1;0;\
                1;0;1;18;\
                ;\"bonus nonstandard\";0;0;\
                \"\";0;0;0\r\n\
                1;42;1;0;32;32;64;\
                ;-32;-32;17;\
                ;\"bonus nonstandard\";1;0;\
                \"hello\n',world\";32;64;17\r\n\
                1;17;1;0;32;32;64;\
                ;-32;-32;17;\
                ;\"bonus nonstandard\";1;0\
                ;\"hello\n',world\";32;64;17\r\n",
        );
        println!("    OK");

        println!("try_csv_to_sql() ..");
        let old = sql;
        let sql = MultiPk::try_csv_to_sql(&csv, Some(b';'));
        assert!(sql.is_ok());
        let sql = sql.unwrap();
        assert_eq!(sql, old);
        println!("    OK");

        println!("try_from_sql() ..");
        let objs = MultiPk::load(db.clone());
        let result = db.clone().exec("DELETE FROM multi_pks", &[]);
        assert!(result.is_ok());
        let result = MultiPk::try_from_sql(db.clone(), &sql);
        eprintln!("{:?}", result);
        assert!(result.is_ok());
        assert_eq!(MultiPk::load(db.clone()), objs);
        let result = MultiPk::try_from_sql(db.clone(), "invalid sql");
        assert!(result.is_err());
        assert_eq!(MultiPk::load(db.clone()), objs);
        println!("    OK");
    }
}
