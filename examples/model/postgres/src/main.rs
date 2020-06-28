#![allow(dead_code)]

use chrono::NaiveDate;
use std::convert::TryInto;
use vicocomo::{DbConn, DbValue, MdlDelete, MdlFind, MdlQueryBld, MdlSave};
use vicocomo_postgres::PgConn;

enum Show {
    Nothing,
    OneLine,
    PrettyUgly,
}
const SHOW: Show = Show::Nothing;

#[derive(
    Clone,
    Debug,
    PartialEq,
    vicocomo::DeleteModel,
    vicocomo::FindModel,
    vicocomo::SaveModel,
)]
struct MultiPk {
    #[vicocomo_optional]
    #[vicocomo_primary]
    id: Option<u32>,
    #[vicocomo_primary]
    id2: u32,
    bool_mand: bool,
    bool_mand_nul: Option<bool>,
    f32_mand: f32,
    #[vicocomo_optional]
    f32_opt: Option<f32>,
    f64_mand: f64,
    #[vicocomo_optional]
    f64_opt_nul: Option<Option<f64>>,
    i32_mand: i32,
    #[vicocomo_optional]
    i32_opt_nul: Option<Option<i32>>,
    i64_mand: i64,
    date_mand: NaiveDate,
    string_mand: String,
    u32_mand: u32,
    u64_mand: u64,
    usize_mand: usize,
}

#[derive(
    Clone,
    Debug,
    vicocomo::DeleteModel,
    vicocomo::FindModel,
    vicocomo::SaveModel,
)]
struct SinglePk {
    #[vicocomo_optional]
    #[vicocomo_primary]
    id: Option<u32>,
    #[vicocomo_order_by(2, "asc")]
    #[vicocomo_optional]
    name: Option<String>,
    data: Option<f32>,
    #[vicocomo_unique = "uni-lbl"]
    un1: i32,
    #[vicocomo_unique = "uni-lbl"]
    #[vicocomo_order_by(1, "desc")]
    un2: i32,
}

pub fn main() {
    dotenv::dotenv().ok();

    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let mut db = PgConn::connect(&database_url).expect("cannot connect");
    match db.connection().batch_execute(
        "
        DROP TABLE IF EXISTS multi_pks;
        DROP TABLE IF EXISTS single_pks;
        CREATE TABLE multi_pks
        (   id               BIGINT NOT NULL DEFAULT 1
        ,   id2              BIGINT
        ,   bool_mand        BIGINT NOT NULL
        ,   bool_mand_nul    BIGINT
        ,   f32_mand         FLOAT(53) NOT NULL
        ,   f32_opt          FLOAT(53) NOT NULL DEFAULT 1.0
        ,   f64_mand         FLOAT(53) NOT NULL
        ,   f64_opt_nul      FLOAT(53) DEFAULT 1.0
        ,   i32_mand         BIGINT NOT NULL
        ,   i32_opt_nul      BIGINT DEFAULT 1
        ,   i64_mand         BIGINT NOT NULL
        ,   date_mand        BIGINT NOT NULL
        ,   string_mand      TEXT NOT NULL
        ,   u32_mand         BIGINT NOT NULL
        ,   u64_mand         BIGINT NOT NULL
        ,   usize_mand       BIGINT NOT NULL
        ,   PRIMARY KEY(id, id2)
        );
        CREATE TABLE single_pks
        (   id    BIGSERIAL PRIMARY KEY
        ,   name  TEXT NOT NULL DEFAULT 'default'
        ,   data  FLOAT(53)
        ,   un1  BIGINT
        ,   un2  BIGINT
        );
    ",
    ) {
        Ok(_) => println!("created tables\n"),
        Err(e) => panic!("{}", e),
    }

    // --- MultiPk -----------------------------------------------------------

    let mut m = MultiPk {
        id: None,
        id2: 1,
        bool_mand: false,
        bool_mand_nul: None,
        f32_mand: 0.0,
        f32_opt: None,
        f64_mand: 0.0,
        f64_opt_nul: None,
        i32_mand: 0,
        i32_opt_nul: None,
        i64_mand: 0,
        date_mand: NaiveDate::from_num_days_from_ce(0),
        string_mand: String::new(),
        u32_mand: 0,
        u64_mand: 0,
        usize_mand: 0,
    };
    println!("inserting {:?} .. ", m);
    assert!(m.insert(&mut db).is_ok());
    assert!(
        format!("{:?}", m)
            == "MultiPk { id: Some(1), id2: 1, bool_mand: false, \
            bool_mand_nul: None, f32_mand: 0.0, f32_opt: Some(1.0), \
            f64_mand: 0.0, f64_opt_nul: Some(Some(1.0)), i32_mand: 0, \
            i32_opt_nul: Some(Some(1)), i64_mand: 0, date_mand: 0000-12-31, \
            string_mand: \"\", u32_mand: 0, u64_mand: 0, usize_mand: 0 }",
    );
    println!("    OK");
    show_multi(&mut db);
    m.id2 = 42;
    println!("not finding non-existing {:?} ..", m);
    assert!(MultiPk::find(&mut db, &(42, 17)).is_none());
    assert!(m.find_equal(&mut db).is_none());
    assert!(
        MultiPk::validate_exists(&mut db, &(m.id2, m.id.unwrap()), "message")
            .err()
            .unwrap()
            .to_string()
            == "Databasfel\nmessage"
    );
    assert!(m.validate_unique(&mut db, "message").is_ok());
    println!("    OK");
    println!("error updating non-existing ..");
    let res = m.update(&mut db);
    assert!(res.is_err());
    println!("    OK");
    println!("inserting non-existing ..");
    let res = m.insert(&mut db);
    assert!(res.is_ok());
    println!("    OK");
    println!("finding existing ..");
    assert!(m == MultiPk::find(&mut db, &(m.id2, m.id.unwrap())).unwrap());
    assert!(m == m.find_equal(&mut db).unwrap());
    assert!(MultiPk::validate_exists(
        &mut db,
        &(m.id2, m.id.unwrap()),
        "message"
    )
    .is_ok());
    assert!(
        m.validate_unique(&mut db, "message")
            .err()
            .unwrap()
            .to_string()
            == "Databasfel\nmessage"
    );
    println!("    OK");
    println!("error inserting existing ..");
    let res = m.insert(&mut db);
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
    m.i64_mand = 64;
    m.date_mand = NaiveDate::from_num_days_from_ce(1);
    m.string_mand = "hello".to_string();
    m.u32_mand = 32;
    m.u64_mand = 64;
    m.usize_mand = 1;
    m.update(&mut db).unwrap();
    assert!(
        format!("{:?}", m)
            == "MultiPk { id: Some(1), id2: 42, bool_mand: true, \
            bool_mand_nul: Some(false), f32_mand: 32.0, f32_opt: Some(32.0), \
            f64_mand: 64.0, f64_opt_nul: Some(None), i32_mand: -32, \
            i32_opt_nul: Some(Some(-32)), i64_mand: 64, \
            date_mand: 0001-01-01, string_mand: \"hello\", u32_mand: 32, \
            u64_mand: 64, usize_mand: 1 }",
    );
    println!("    OK");
    println!("finding existing ..");
    let res = m.find_equal(&mut db);
    assert!(res.is_some());
    assert!(res.unwrap() == m);
    println!("    OK");
    println!("deleting existing {:?} ..", m);
    let res = m.clone().delete(&mut db);
    assert!(res.is_ok());
    assert!(res.unwrap() == 1);
    println!("    OK");
    println!("error deleting non-existing {:?}", m);
    let res = m.delete(&mut db);
    assert!(res.is_err());
    println!("    OK");

    // --- SinglePk ----------------------------------------------------------

    let mut s = SinglePk {
        id: None,
        name: None,
        data: Some(17f32),
        un1: 2,
        un2: 1,
    };
    println!("inserting {:?} ..", s);
    assert!(s.insert(&mut db).is_ok());
    assert!(format!("{:?}", s) ==
        "SinglePk { id: Some(1), name: Some(\"default\"), data: Some(17.0), \
            un1: 2, un2: 1 }",
    );
    println!("    OK");
    show_single(&mut db);
    let ss = vec![
        SinglePk {
            id: None,
            name: Some(String::from("hej")),
            data: None,
            un1: 1,
            un2: 1,
        },
        SinglePk {
            id: None,
            name: Some(String::from("hopp")),
            data: None,
            un1: 1,
            un2: 2,
        },
    ];
    println!("inserting batch {:?} ..", ss);
    let res = SinglePk::insert_batch(&mut db, &ss[..]);
    assert!(res.is_ok());
    assert!(format!("{:?}", res) ==
        "Ok([SinglePk { id: Some(2), name: Some(\"hej\"), data: None, \
            un1: 1, un2: 1 }, \
            SinglePk { id: Some(3), name: Some(\"hopp\"), data: None, \
            un1: 1, un2: 2 }])"
    );
    println!("    OK");
    show_single(&mut db);
    s = SinglePk {
        id: Some(42),
        name: Some(String::from("hej")),
        data: None,
        un1: 1,
        un2: 42,
    };
    println!("not finding non-existing {:?} ..", s);
    let res = s.find_equal(&mut db);
    assert!(res.is_none());
    println!("    OK");
    println!("not finding non-existing by unique fields ..");
    assert!(SinglePk::find_by_un1_and_un2(&mut db, s.un1, s.un2).is_none());
    assert!(s.find_equal_un1_and_un2(&mut db).is_none());
    assert!(
        SinglePk::validate_exists_un1_and_un2(
            &mut db, s.un1, s.un2, "message"
        )
        .err()
        .unwrap()
        .to_string()
            == "Databasfel\nmessage: 1, 42"
    );
    assert!(s.validate_unique_un1_and_un2(&mut db, "message").is_ok());
    println!("    OK");
    println!("error updating non-existing ..");
    let res = s.update(&mut db);
    assert!(res.is_err());
    println!("    OK");
    {
        println!(",- transaction begin ------------------------------------");
        let mut trans = db.transaction().unwrap();
        println!("| inserting non-existing ..");
        let res = s.insert(&mut trans);
        assert!(res.is_ok());
        assert!(format!("{:?}", s) ==
            "SinglePk { id: Some(42), name: Some(\"hej\"), data: None, \
                un1: 1, un2: 42 }"
        );
        let mut un2 = 1000;
        let mut name = "aaa".to_string();
        for s in SinglePk::load(&mut trans).unwrap() {
            assert!(s.un2 <= un2);
            if s.un2 == un2 {
                assert!(s.name.clone().unwrap() >= name);
            }
            un2 = s.un2;
            name = s.name.unwrap().clone();
        }
        println!("|   OK");
        show_single(&mut trans);
        s.name = Some("nytt namn".to_string());
        println!("| updating existing {:?} ..", s);
        let res = s.update(&mut trans);
        assert!(res.is_ok());
        assert!(format!("{:?}", s) ==
            "SinglePk { id: Some(42), name: Some(\"nytt namn\"), data: None, \
                un1: 1, un2: 42 }"
        );
        println!("|   OK");
        Box::new(trans).commit().unwrap();
        println!("'- transaction commit -----------------------------------");
        assert!(s.find_equal(&mut db).is_some());
        println!("    OK");
    }
    show_single(&mut db);
    println!("error inserting existing ..");
    let res = s.insert(&mut db);
    assert!(res.is_err());
    println!("    OK");
    println!("finding existing ..");
    let res = s.find_equal(&mut db);
    assert!(res.is_some());
    assert!(format!("{:?}", res.unwrap()) ==
        "SinglePk { id: Some(42), name: Some(\"nytt namn\"), data: None, \
            un1: 1, un2: 42 }"
    );
    println!("    OK");
    println!("finding existing by unique fields ..");
    let res = SinglePk::find_by_un1_and_un2(&mut db, s.un1, s.un2);
    assert!(res.is_some());
    let res = res.unwrap();
    assert!(format!("{:?}", &res) ==
        "SinglePk { id: Some(42), name: Some(\"nytt namn\"), data: None, \
            un1: 1, un2: 42 }"
    );
    assert!(
        format!("{:?}", &res)
            == format!("{:?}", &s.find_equal_un1_and_un2(&mut db).unwrap())
    );
    assert!(SinglePk::validate_exists_un1_and_un2(
        &mut db, s.un1, s.un2, "message"
    )
    .is_ok());
    assert!(
        s.validate_unique_un1_and_un2(&mut db, "message")
            .err()
            .unwrap()
            .to_string()
            == "Databasfel\nmessage: 1, 42"
    );
    println!("    OK");
    let query = MdlQueryBld::new()
        .col("un2")
        .eq(Some(&DbValue::Int(1)))
        .or("name")
        .gt(Some(&DbValue::NulText(Some(String::from("hopp")))))
        .query()
        .unwrap();
    println!("query() with default order ..");
    let found = SinglePk::query(&mut db, &query);
    assert!(
        format!("{:?}", found.unwrap())
            == "[\
            SinglePk { \
                id: Some(42), \
                name: Some(\"nytt namn\"), \
                data: None, \
                un1: 1, \
                un2: 42 \
            }, \
            SinglePk { \
                id: Some(1), \
                name: Some(\"default\"), \
                data: Some(17.0), \
                un1: 2, \
                un2: 1 \
            }, \
            SinglePk { \
                id: Some(2), \
                name: Some(\"hej\"), \
                data: None, \
                un1: 1, \
                un2: 1 \
            }\
        ]"
    );
    println!("    OK");
    println!("query() with custom order ..");
    let mut query = MdlQueryBld::new()
        .col("un2")
        .eq(Some(&DbValue::Int(1)))
        .or("name")
        .gt(Some(&DbValue::NulText(Some(String::from("hopp")))))
        .order("un1, name DESC")
        .query()
        .unwrap();
    let found = SinglePk::query(&mut db, &query);
    assert!(
        format!("{:?}", found.unwrap())
            == "[\
            SinglePk { \
                id: Some(42), \
                name: Some(\"nytt namn\"), \
                data: None, \
                un1: 1, \
                un2: 42 \
            }, \
            SinglePk { \
                id: Some(2), \
                name: Some(\"hej\"), \
                data: None, \
                un1: 1, \
                un2: 1 \
            }, \
            SinglePk { \
                id: Some(1), \
                name: Some(\"default\"), \
                data: Some(17.0), \
                un1: 2, \
                un2: 1 \
            }\
        ]"
    );
    println!("    OK");
    println!("query() without filter with custom order ..");
    query = MdlQueryBld::new().order("un1, name DESC").query().unwrap();
    let found = SinglePk::query(&mut db, &query);
    assert!(
        format!("{:?}", found.unwrap())
            == "[\
            SinglePk { \
                id: Some(42), \
                name: Some(\"nytt namn\"), \
                data: None, \
                un1: 1, \
                un2: 42 \
            }, \
            SinglePk { \
                id: Some(3), \
                name: Some(\"hopp\"), \
                data: None, \
                un1: 1, \
                un2: 2 \
            }, \
            SinglePk { \
                id: Some(2), \
                name: Some(\"hej\"), \
                data: None, \
                un1: 1, \
                un2: 1 \
            }, \
            SinglePk { \
                id: Some(1), \
                name: Some(\"default\"), \
                data: Some(17.0), \
                un1: 2, \
                un2: 1 \
            }\
        ]"
    );
    println!("    OK");
    println!("query() without filter with limit ..");
    query.set_limit(Some(2));
    let found = SinglePk::query(&mut db, &query);
    assert!(
        format!("{:?}", found.unwrap())
            == "[\
            SinglePk { \
                id: Some(42), \
                name: Some(\"nytt namn\"), \
                data: None, \
                un1: 1, \
                un2: 42 \
            }, \
            SinglePk { \
                id: Some(3), \
                name: Some(\"hopp\"), \
                data: None, \
                un1: 1, \
                un2: 2 \
            }\
        ]"
    );
    println!("    OK");
    println!("query() without filter with offset ..");
    query.set_limit(None);
    query.set_offset(Some(1));
    let found = SinglePk::query(&mut db, &query);
    assert!(
        format!("{:?}", found.unwrap())
            == "[\
            SinglePk { \
                id: Some(3), \
                name: Some(\"hopp\"), \
                data: None, \
                un1: 1, \
                un2: 2 \
            }, \
            SinglePk { \
                id: Some(2), \
                name: Some(\"hej\"), \
                data: None, \
                un1: 1, \
                un2: 1 \
            }, \
            SinglePk { \
                id: Some(1), \
                name: Some(\"default\"), \
                data: Some(17.0), \
                un1: 2, \
                un2: 1 \
            }\
        ]"
    );
    println!("    OK");
    println!("query() without filter with limit and offset ..");
    query.set_limit(Some(2));
    query.set_offset(Some(1));
    let found = SinglePk::query(&mut db, &query);
    assert!(
        format!("{:?}", found.unwrap())
            == "[\
            SinglePk { \
                id: Some(3), \
                name: Some(\"hopp\"), \
                data: None, \
                un1: 1, \
                un2: 2 \
            }, \
            SinglePk { \
                id: Some(2), \
                name: Some(\"hej\"), \
                data: None, \
                un1: 1, \
                un2: 1 \
            }\
        ]"
    );
    println!("    OK");
    {
        println!(",- transaction begin ------------------------------------");
        let mut trans = db.transaction().unwrap();
        println!("| deleting existing {:?} ..", s);
        let res = s.clone().delete(&mut trans);
        assert!(res.is_ok());
        assert!(res.unwrap() == 1);
        println!("|   OK");
        show_single(&mut trans);
        println!("| error deleting non-existing {:?}", s);
        let res = s.clone().delete(&mut trans);
        assert!(res.is_err());
        println!("|   OK");
        Box::new(trans).rollback().unwrap();
        assert!(s.find_equal(&mut db).is_some());
        println!("'- transaction rollback ---------------------------------");
        println!("    OK");
    }
    println!("deleting existing {:?} ..", s);
    let res = s.clone().delete(&mut db);
    assert!(res.is_ok());
    assert!(res.unwrap() == 1);
    println!("    OK");
    for (pks, del) in [([42, 43], 0), ([42, 3], 1), ([2, 1], 2)].iter() {
        println!("deleting {} out of batch {:?}", del, pks);
        let res = SinglePk::delete_batch(&mut db, pks);
        assert!(res.is_ok());
        assert!(res.unwrap() == *del);
        println!("    OK");
        show_single(&mut db);
    }

    // --- The End -----------------------------------------------------------

    db.connection()
        .batch_execute(
            "
            DROP TABLE multi_pks;
            DROP TABLE single_pks;
        ",
        )
        .unwrap();
}

fn show_multi<'a>(db: &mut impl DbConn<'a>) {
    match SHOW {
        Show::Nothing => (),
        Show::OneLine => {
            println!("--- multi_pks: {:?}", MultiPk::load(db).unwrap())
        }
        Show::PrettyUgly => {
            println!("--- multi_pks: {:#?}", MultiPk::load(db).unwrap())
        }
    }
}

fn show_single<'a>(db: &mut impl DbConn<'a>) {
    match SHOW {
        Show::Nothing => (),
        Show::OneLine => {
            println!("--- single_pks: {:?}", SinglePk::load(db).unwrap())
        }
        Show::PrettyUgly => {
            println!("--- single_pks: {:#?}", SinglePk::load(db).unwrap())
        }
    }
}
