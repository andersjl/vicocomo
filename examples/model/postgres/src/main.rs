// TODO: test optional unique field without value

#![allow(dead_code)]

enum Show {
    Nothing,
    OneLine,
    PrettyUgly,
}
const SHOW: Show = Show::Nothing;

mod models {

    pub mod multi_pk {
        use chrono::NaiveDate;

        #[derive(
            Clone,
            Debug,
            PartialEq,
            ::vicocomo::BelongsTo,
            ::vicocomo::Delete,
            ::vicocomo::Find,
            ::vicocomo::Save,
        )]
        pub struct MultiPk {
            #[vicocomo_optional]
            #[vicocomo_primary]
            pub id: Option<u32>,
            #[vicocomo_primary]
            pub id2: u32,
            pub bool_mand: bool,
            pub bool_mand_nul: Option<bool>,
            pub f32_mand: f32,
            #[vicocomo_optional]
            pub f32_opt: Option<f32>,
            pub f64_mand: f64,
            #[vicocomo_optional]
            pub f64_opt_nul: Option<Option<f64>>,
            pub i32_mand: i32,
            #[vicocomo_optional]
            pub i32_opt_nul: Option<Option<i32>>,
            #[vicocomo_belongs_to()]
            pub default_parent_id: i64,
            #[vicocomo_belongs_to(
                remote_pk = "pk mandatory",
                remote_type = "crate::models::other_parent::NonstandardParent"
            )]
            pub other_parent_id: Option<String>,
            #[vicocomo_belongs_to(
                name = "BonusParent",
                remote_pk = "pk mandatory",
                remote_type = "crate::models::other_parent::NonstandardParent"
            )]
            pub bonus_parent: String,
            pub date_mand: NaiveDate,
            pub string_mand: String,
            pub u32_mand: u32,
            pub u64_mand: u64,
            pub usize_mand: usize,
        }
    }

    pub mod single_pk {
        #[derive(
            Clone,
            Debug,
            ::vicocomo::Delete,
            ::vicocomo::Find,
            ::vicocomo::Save,
        )]
        pub struct SinglePk {
            #[vicocomo_optional]
            #[vicocomo_primary]
            pub id: Option<u32>,
            #[vicocomo_order_by(2, "asc")]
            #[vicocomo_optional]
            pub name: Option<String>,
            pub data: Option<f32>,
            #[vicocomo_optional]
            #[vicocomo_unique = "uni-lbl"]
            pub un1: Option<i32>,
            #[vicocomo_unique = "uni-lbl"]
            #[vicocomo_order_by(1, "desc")]
            pub un2: i32,
        }
    }

    pub mod default_parent {
        #[derive(
            Clone,
            Debug,
            vicocomo::Delete,
            vicocomo::Find,
            vicocomo::HasMany,
            vicocomo::Save,
        )]
        #[vicocomo_has_many(on_delete = "cascade", remote_type = "MultiPk")]
        #[vicocomo_has_many(remote_type = "SinglePk", join_table = "joins")]
        pub struct DefaultParent {
            #[vicocomo_optional]
            #[vicocomo_primary]
            pub id: Option<i64>,
            pub name: String,
        }
    }

    pub mod other_parent {
        #[derive(
            Clone,
            Debug,
            vicocomo::BelongsTo,
            vicocomo::Delete,
            vicocomo::Find,
            vicocomo::HasMany,
            vicocomo::Save,
        )]
        #[vicocomo_has_many(
            on_delete = "forget",
            remote_fk_col = "other_parent_id",
            remote_type = "MultiPk",
        )]
        #[vicocomo_has_many(
            name = "BonusChild",
            remote_fk_col = "bonus_parent",
            remote_type = "MultiPk",
        )]
        #[vicocomo_has_many(
            remote_type = "crate::models::other_parent::NonstandardParent",
        )]
        pub struct NonstandardParent {
            #[vicocomo_primary]
            pub pk: String,
            #[vicocomo_belongs_to(
                remote_pk = "pk mandatory",
                remote_type = "crate::models::other_parent::NonstandardParent"
            )]
            pub nonstandard_parent_id: Option<String>,
        }
    }
}

use ::vicocomo::{DbConn, DbValue, Delete, Find, QueryBld, Save};
use ::vicocomo_postgres::PgConn;
use chrono::NaiveDate;
use models::{
    default_parent::DefaultParent, multi_pk::MultiPk,
    other_parent::NonstandardParent, single_pk::SinglePk,
};

#[tokio::main]
async fn main() {
    use futures::executor::block_on;

    dotenv::dotenv().ok();
    let (pg_client, pg_conn) = block_on(tokio_postgres::connect(
        &std::env::var("DATABASE_URL").expect("DATABASE_URL must be set"),
        tokio_postgres::NoTls,
    ))
    .expect("cannot connect");
    tokio::spawn(async move {
        if let Err(e) = pg_conn.await {
            eprintln!("connection error: {}", e);
        }
    });
    match block_on(pg_client.batch_execute(
        " DROP TABLE IF EXISTS joins
        ; DROP TABLE IF EXISTS multi_pks
        ; DROP TABLE IF EXISTS single_pks
        ; DROP TABLE IF EXISTS default_parents
        ; DROP TABLE IF EXISTS nonstandard_parents
        ; CREATE TABLE multi_pks
        (   id             BIGINT NOT NULL DEFAULT 1
        ,   id2            BIGINT
        ,   bool_mand      BIGINT NOT NULL
        ,   bool_mand_nul  BIGINT
        ,   f32_mand       FLOAT(53) NOT NULL
        ,   f32_opt        FLOAT(53) NOT NULL DEFAULT 1.0
        ,   f64_mand       FLOAT(53) NOT NULL
        ,   f64_opt_nul    FLOAT(53) DEFAULT 1.0
        ,   i32_mand       BIGINT NOT NULL
        ,   i32_opt_nul    BIGINT DEFAULT 1
        ,   default_parent_id  BIGINT NOT NULL
        ,   other_parent_id    TEXT
        ,   bonus_parent   TEXT NOT NULL
        ,   date_mand      BIGINT NOT NULL
        ,   string_mand    TEXT NOT NULL
        ,   u32_mand       BIGINT NOT NULL
        ,   u64_mand       BIGINT NOT NULL
        ,   usize_mand     BIGINT NOT NULL
        ,   PRIMARY KEY(id, id2)
        )
        ; CREATE TABLE single_pks
        (   id    BIGSERIAL PRIMARY KEY
        ,   name  TEXT NOT NULL DEFAULT 'default'
        ,   data  FLOAT(53)
        ,   un1   BIGINT DEFAULT 4711
        ,   un2   BIGINT NOT NULL
        ,   UNIQUE(un1, un2)
        )
        ; CREATE TABLE joins
        ( default_parent_id  BIGINT NOT NULL
        , single_pk_id       BIGINT NOT NULL
        , PRIMARY KEY(default_parent_id, single_pk_id)
        )
        ; CREATE TABLE default_parents
        (   id    BIGSERIAL PRIMARY KEY
        ,   name  TEXT NOT NULL
        )
        ; CREATE TABLE nonstandard_parents
        (   pk                     TEXT PRIMARY KEY
        ,   nonstandard_parent_id  TEXT
        )
        ; INSERT INTO default_parents (name)
            VALUES ('default filler'), ('used default')
        ; INSERT INTO nonstandard_parents (pk, nonstandard_parent_id)
            VALUES ('nonstandard', NULL) , ('bonus nonstandard', NULL)
        ;
    ",
    )) {
        Ok(_) => println!("created tables\n"),
        Err(e) => panic!("{}", e),
    }
    let db = PgConn::new(pg_client);

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
        default_parent_id: 2,
        other_parent_id: None,
        bonus_parent: "bonus nonstandard".to_string(),
        date_mand: NaiveDate::from_num_days_from_ce(0),
        string_mand: String::new(),
        u32_mand: 0,
        u64_mand: 0,
        usize_mand: 0,
    };

    // - - inserting, finding, and updating  - - - - - - - - - - - - - - - - -

    println!("inserting {:?} .. ", m);
    assert!(m.insert(&db).is_ok());
    assert!(
        format!("{:?}", m)
            == "MultiPk { id: Some(1), id2: 1, bool_mand: false, \
            bool_mand_nul: None, f32_mand: 0.0, f32_opt: Some(1.0), \
            f64_mand: 0.0, f64_opt_nul: Some(Some(1.0)), i32_mand: 0, \
            i32_opt_nul: Some(Some(1)), default_parent_id: 2, \
            other_parent_id: None, bonus_parent: \"bonus nonstandard\", \
            date_mand: 0000-12-31, string_mand: \"\", u32_mand: 0, \
            u64_mand: 0, usize_mand: 0 }",
    );
    println!("    OK");
    show_multi(&db);
    m.id2 = 42;
    println!("not finding non-existing {:?} ..", m);
    assert!(MultiPk::find(&db, &(42, 17)).is_none());
    assert!(m.find_equal(&db).is_none());
    assert!(
        MultiPk::validate_exists(&db, &(m.id2, m.id.unwrap()), "message")
            .err()
            .unwrap()
            .to_string()
            == "Database error\nmessage"
    );
    assert!(m.validate_unique(&db, "message").is_ok());
    println!("    OK");
    println!("error updating non-existing ..");
    let res = m.update(&db);
    assert!(res.is_err());
    println!("    OK");
    println!("inserting non-existing ..");
    let res = m.insert(&db);
    assert!(res.is_ok());
    println!("    OK");
    println!("finding existing ..");
    assert!(m == MultiPk::find(&db, &(m.id.unwrap(), m.id2)).unwrap());
    assert!(m == m.find_equal(&db).unwrap());
    assert!(MultiPk::validate_exists(
        &db,
        &(m.id.unwrap(), m.id2),
        "message"
    )
    .is_ok());
    assert!(
        m.validate_unique(&db, "message").err().unwrap().to_string()
            == "Database error\nmessage"
    );
    println!("    OK");
    println!("error inserting existing ..");
    let res = m.insert(&db);
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
    m.update(&db).unwrap();
    assert!(
        format!("{:?}", m)
            == "MultiPk { id: Some(1), id2: 42, bool_mand: true, \
            bool_mand_nul: Some(false), f32_mand: 32.0, f32_opt: Some(32.0), \
            f64_mand: 64.0, f64_opt_nul: Some(None), i32_mand: -32, \
            i32_opt_nul: Some(Some(-32)), default_parent_id: 1, \
            other_parent_id: None, bonus_parent: \"bonus nonstandard\", \
            date_mand: 0001-01-01, string_mand: \"hello\", u32_mand: 32, \
            u64_mand: 64, usize_mand: 1 }",
    );
    println!("    OK");
    println!("save() existing after change ..");
    m.usize_mand = 17;
    assert!(m.save(&db).is_ok());
    println!("    OK");
    println!("finding existing ..");
    let res = m.find_equal(&db);
    assert!(res.is_some());
    assert!(res.unwrap() == m);
    println!("    OK");
    println!("save() non-existing ..");
    let mut m2 = m.clone();
    m2.id2 = 17;
    m2.default_parent_id = 1;
    assert!(m2.save(&db).is_ok());
    assert!(m2.find_equal(&db).unwrap() == m2);
    println!("    OK");

    // - - belongs-to association  - - - - - - - - - - - - - - - - - - - - - -

    println!("setting saved parent ..");
    assert!(m
        .belong_to_default_parent(&DefaultParent::find(&db, &2).unwrap(),)
        .is_ok(),);
    assert!(m.default_parent_id == 2);
    let np = &NonstandardParent::find(&db, &"nonstandard".to_string())
        .unwrap();
    assert!(m.belong_to_nonstandard_parent(np).is_ok());
    assert!(m.other_parent_id == Some("nonstandard".to_string()));
    let bp =
        &mut NonstandardParent::find(&db, &"bonus nonstandard".to_string())
            .unwrap();
    assert!(m.belong_to_bonus_parent(bp).is_ok());
    assert!(m.bonus_parent == "bonus nonstandard");
    assert!(bp.belong_to_nonstandard_parent(np).is_ok());
    assert!(bp.nonstandard_parent_id == Some("nonstandard".to_string()));
    assert!(m.save(&db).is_ok());
    assert!(bp.save(&db).is_ok());
    println!("    OK");
    println!("unsetting parent ..");
    assert!(m.belong_to_no_nonstandard_parent().is_ok());
    assert!(m.other_parent_id.is_none());
    assert!(m.save(&db).is_ok());
    println!("    OK");
    println!("error setting unsaved parent ..");
    assert!(m
        .belong_to_default_parent(&DefaultParent {
            id: None,
            name: "unsaved".to_string(),
        })
        .is_err());
    assert!(m.default_parent_id == 2);
    println!("    OK");
    println!("getting saved parent ..");
    let dp = m.belongs_to_default_parent(&db);
    assert!(dp.is_some());
    let dp = dp.unwrap();
    assert!(
        format!("{:?}", dp)
            == "DefaultParent { id: Some(2), name: \"used default\" }"
    );
    m.belong_to_nonstandard_parent(np)
        .and_then(|()| m.save(&db))
        .unwrap();
    let np = m.belongs_to_nonstandard_parent(&db);
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
    let dp_sibs = m.default_parent_siblings(&db);
    assert!(dp_sibs.is_ok());
    let dp_sibs = dp_sibs.unwrap();
    assert!(dp_sibs.len() == 2);
    assert!(dp_sibs.iter().filter(|s| s.default_parent_id == 2).count() == 2);
    let np_sibs: Result<Vec<MultiPk>, ::vicocomo::Error> =
        MultiPk::all_belonging_to_nonstandard_parent(&db, &np);
    assert!(np_sibs.is_ok());
    let np_sibs = np_sibs.unwrap();
    assert!(np_sibs.len() == 1);
    let grown_sibs: Result<Vec<NonstandardParent>, ::vicocomo::Error> =
        NonstandardParent::all_belonging_to_nonstandard_parent(&db, &np);
    assert!(grown_sibs.is_ok());
    let grown_sibs = grown_sibs.unwrap();
    assert!(grown_sibs.len() == 1);
    println!("    OK");

    // - - one-to-many association - - - - - - - - - - - - - - - - - - - - - -

    show_multi(&db);
    println!("finding children ..");
    let dp_chn = dp.find_remote_multi_pk(&db, None);
    assert!(dp_chn.is_ok());
    let dp_chn = dp_chn.unwrap();
    assert!(format!("{:?}", dp_chn) == format!("{:?}", dp_sibs));
    let bp_chn = bp.find_remote_bonus_child(&db, None);
    assert!(bp_chn.is_ok());
    let bp_chn = bp_chn.unwrap();
    assert!(
        format!("{:?}", bp_chn)
            == format!("{:?}", MultiPk::load(&db).unwrap()),
    );
    let grown_chn = np.find_remote_nonstandard_parent(&db, None);
    assert!(grown_chn.is_ok());
    let grown_chn = grown_chn.unwrap();
    assert!(format!("{:?}", grown_chn) == format!("{:?}", grown_sibs));
    println!("    OK");

    // - - deleting  - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

    println!("deleting existing {:?} ..", m);
    let res = m.clone().delete(&db);
    assert!(res.is_ok());
    assert!(res.unwrap() == 1);
    println!("    OK");
    println!("error deleting non-existing {:?}", m);
    let res = m.delete(&db);
    assert!(res.is_err());
    println!("    OK");
    println!("error deleting restricted parent");
    let mut m = MultiPk::find(&db, &(1, 17)).unwrap();
    m.belong_to_nonstandard_parent(&np)
        .and_then(|()| m.save(&db))
        .unwrap();
    let old_counts = (
        np.find_remote_multi_pk(&db, None).unwrap().len(),
        np.find_remote_nonstandard_parent(&db, None).unwrap().len(),
    );
    let res = np.clone().delete(&db);
    assert!(res.is_err());
    let new_counts = (
        np.find_remote_multi_pk(&db, None).unwrap().len(),
        np.find_remote_nonstandard_parent(&db, None).unwrap().len(),
    );
    assert!(new_counts == old_counts);
    println!("    OK");

    // --- SinglePk ----------------------------------------------------------

    // - - inserting - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

    let mut s = SinglePk {
        id: None,
        name: None,
        data: Some(17f32),
        un1: Some(2),
        un2: 1,
    };
    println!("inserting {:?} ..", s);
    assert!(s.insert(&db).is_ok());
    assert!(format!("{:?}", s) ==
        "SinglePk { id: Some(1), name: Some(\"default\"), data: Some(17.0), \
            un1: Some(2), un2: 1 }",
    );
    println!("    OK");
    show_single(&db);
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
    let res = SinglePk::insert_batch(&db, &ss[..]);
    assert!(res.is_ok());
    assert!(format!("{:?}", res) ==
        "Ok([SinglePk { id: Some(2), name: Some(\"hej\"), data: None, \
            un1: Some(1), un2: 1 }, \
            SinglePk { id: Some(3), name: Some(\"hopp\"), data: None, \
            un1: Some(1), un2: 2 }])"
    );
    println!("    OK");
    show_single(&db);
    s = SinglePk {
        id: Some(42),
        name: Some(String::from("hej")),
        data: None,
        un1: Some(1),
        un2: 42,
    };

    // - - not finding or updating non-existing  - - - - - - - - - - - - - - -

    println!("not finding non-existing {:?} ..", s);
    let res = s.find_equal(&db);
    assert!(res.is_none());
    println!("    OK");
    println!("not finding non-existing by unique fields ..");
    assert!(
        SinglePk::find_by_un1_and_un2(&db, s.un1.unwrap(), s.un2).is_none()
    );
    assert!(s.find_equal_un1_and_un2(&db).is_none());
    assert!(
        SinglePk::validate_exists_un1_and_un2(
            &db,
            s.un1.unwrap(),
            s.un2,
            "message"
        )
        .err()
        .unwrap()
        .to_string()
            == "Database error\nmessage: 1, 42"
    );
    assert!(s.validate_unique_un1_and_un2(&db, "message").is_ok());
    println!("    OK");
    println!("error updating non-existing ..");
    let res = s.update(&db);
    assert!(res.is_err());
    println!("    OK");

    // - - commit transaction  - - - - - - - - - - - - - - - - - - - - - - - -

    println!(",- transaction begin ------------------------------------");
    db.begin().unwrap();
    println!("| inserting non-existing ..");
    let res = s.insert(&db);
    assert!(res.is_ok());
    assert!(
        format!("{:?}", s)
            == "SinglePk { id: Some(42), name: Some(\"hej\"), data: None, \
            un1: Some(1), un2: 42 }"
    );
    let mut un2 = 1000;
    let mut name = "aaa".to_string();
    for s in SinglePk::load(&db).unwrap() {
        assert!(s.un2 <= un2);
        if s.un2 == un2 {
            assert!(s.name.clone().unwrap() >= name);
        }
        un2 = s.un2;
        name = s.name.unwrap().clone();
    }
    println!("|   OK");
    show_single(&db);
    s.name = Some("nytt namn".to_string());
    println!("| updating existing {:?} ..", s);
    let res = s.update(&db);
    assert!(res.is_ok());
    assert!(format!("{:?}", s) ==
        "SinglePk { id: Some(42), name: Some(\"nytt namn\"), data: None, \
            un1: Some(1), un2: 42 }"
    );
    println!("|   OK");
    db.commit().unwrap();
    println!("'- transaction commit -----------------------------------");
    assert!(s.find_equal(&db).is_some());
    println!("    OK");
    show_single(&db);
    println!("error inserting existing ..");
    let res = s.insert(&db);
    assert!(res.is_err());
    println!("    OK");

    // - - finding existing  - - - - - - - - - - - - - - - - - - - - - - - - -

    println!("finding existing ..");
    let res = s.find_equal(&db);
    assert!(res.is_some());
    assert!(format!("{:?}", res.unwrap()) ==
        "SinglePk { id: Some(42), name: Some(\"nytt namn\"), data: None, \
            un1: Some(1), un2: 42 }"
    );
    println!("    OK");
    println!("finding existing by unique fields ..");
    let res = SinglePk::find_by_un1_and_un2(&db, s.un1.unwrap(), s.un2);
    assert!(res.is_some());
    let res = res.unwrap();
    assert!(format!("{:?}", &res) ==
        "SinglePk { id: Some(42), name: Some(\"nytt namn\"), data: None, \
            un1: Some(1), un2: 42 }"
    );
    assert!(
        format!("{:?}", &res)
            == format!("{:?}", &s.find_equal_un1_and_un2(&db).unwrap())
    );
    assert!(SinglePk::validate_exists_un1_and_un2(
        &db,
        s.un1.unwrap(),
        s.un2,
        "message"
    )
    .is_ok());
    assert!(
        s.validate_unique_un1_and_un2(&db, "message")
            .err()
            .unwrap()
            .to_string()
            == "Database error\nmessage: Some(1), 42"
    );
    println!("    OK");

    // - - query() - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

    let query = QueryBld::new()
        .col("un2")
        .eq(Some(&DbValue::Int(1)))
        .or("name")
        .gt(Some(&DbValue::NulText(Some(String::from("hopp")))))
        .query()
        .unwrap();
    println!("query() with default order ..");
    let found = SinglePk::query(&db, &query);
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
    let found = SinglePk::query(&db, &query);
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
    let found = SinglePk::query(&db, &query);
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
    let found = SinglePk::query(&db, &query);
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
    let found = SinglePk::query(&db, &query);
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
    let found = SinglePk::query(&db, &query);
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

    // - - roll back transaction - - - - - - - - - - - - - - - - - - - - - - -

    {
        println!(",- transaction begin ------------------------------------");
        db.begin().unwrap();
        println!("| deleting existing {:?} ..", s);
        let res = s.clone().delete(&db);
        assert!(res.is_ok());
        assert!(res.unwrap() == 1);
        println!("|   OK");
        show_single(&db);
        println!("| error deleting non-existing {:?}", s);
        let res = s.clone().delete(&db);
        assert!(res.is_err());
        println!("|   OK");
        db.rollback().unwrap();
        assert!(s.find_equal(&db).is_some());
        println!("'- transaction rollback ---------------------------------");
        println!("    OK");
    }

    // - - deleting  - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

    println!("deleting existing {:?} ..", s);
    let res = s.clone().delete(&db);
    assert!(res.is_ok());
    assert!(res.unwrap() == 1);
    println!("    OK");
    for (pks, del) in [([42, 43], 0), ([42, 3], 1), ([2, 1], 2)].iter() {
        println!("deleting {} out of batch {:?}", del, pks);
        let res = SinglePk::delete_batch(&db, pks);
        assert!(res.is_ok());
        assert!(res.unwrap() == *del);
        println!("    OK");
        show_single(&db);
    }

    // - - many-to-many association  - - - - - - - - - - - - - - - - - - - - -

    println!("many-to-many ..");
    let mut pa = DefaultParent {
        id: None,
        name: "parent-a".to_string(),
    };
    pa.save(&db).unwrap();
    let mut pb = DefaultParent {
        id: None,
        name: "parent-b".to_string(),
    };
    pb.save(&db).unwrap();
    let mut sa = SinglePk {
        id: None,
        name: Some("child-a".to_string()),
        data: None,
        un1: None,
        un2: 101,
    };
    sa.save(&db).unwrap();
    let mut sb = SinglePk {
        id: None,
        name: Some("child-b".to_string()),
        data: None,
        un1: None,
        un2: 102,
    };
    sb.save(&db).unwrap();
    assert!(pa.connect_to_single_pk(&db, &sa).is_ok());
    assert!(pa.connect_to_single_pk(&db, &sa).is_err());
    assert!(pa.connect_to_single_pk(&db, &sb).is_ok());
    assert!(pb.connect_to_single_pk(&db, &sb).is_ok());
    assert!(pa.find_remote_single_pk(&db, None).unwrap().len() == 2);
    let pa_sb_assoc =
        "Ok([SinglePk { id: Some(5), name: Some(\"child-b\"), data: None, \
        un1: Some(4711), un2: 102 }])";
    assert!(
        format!(
            "{:?}",
            pa.find_remote_single_pk(
                &db,
                QueryBld::new()
                    .col("name")
                    .eq(Some(&DbValue::Text("child-b".to_string())))
                    .query()
                    .as_ref(),
            ),
        ) == pa_sb_assoc
    );
    assert!(
        format!("{:?}", pb.disconnect_from_single_pk(&db, &sa)) == "Ok(0)"
    );
    assert!(
        format!("{:?}", pb.disconnect_from_single_pk(&db, &sb)) == "Ok(1)"
    );
    assert!(
        format!("{:?}", pa.disconnect_from_single_pk(&db, &sa)) == "Ok(1)"
    );
    assert!(
        format!("{:?}", pa.find_remote_single_pk(&db, None)) == pa_sb_assoc
    );
    println!("    OK");
}

fn show_multi<'a>(db: &impl DbConn) {
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

fn show_single<'a>(db: &impl DbConn) {
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
