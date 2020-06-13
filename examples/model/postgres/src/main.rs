#![allow(dead_code)]

use std::convert::TryInto;
use vicocomo::{DbConn, Delete, Find, Save};
use vicocomo_postgres::PgConn;

enum Show{Nothing, OneLine, PrettyUgly}
const SHOW: Show = Show::Nothing;

#[derive(
    Clone,
    Debug,
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
    #[vicocomo_optional]
    name: Option<String>,
    data: Option<f32>,
    #[vicocomo_unique = "uni-lbl"]
    un1: i32,
    #[vicocomo_unique = "uni-lbl"]
    un2: i32,
}

#[derive(
    Clone,
    Debug,
    vicocomo::DeleteModel,
    vicocomo::FindModel,
    vicocomo::SaveModel
)]
struct SinglePk {
    #[vicocomo_optional]
    #[vicocomo_primary]
    id: Option<u32>,
    #[vicocomo_optional]
    name: Option<String>,
    data: Option<f32>,
    #[vicocomo_unique = "uni-lbl"]
    un1: i32,
    #[vicocomo_unique = "uni-lbl"]
    un2: i32,
}

pub fn main() {

    dotenv::dotenv().ok();

    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let mut db = PgConn::connect(&database_url).expect("cannot connect");
    match db.connection().batch_execute("
        DROP TABLE IF EXISTS multi_pks;
        DROP TABLE IF EXISTS single_pks;
        CREATE TABLE multi_pks
        (   id    BIGINT NOT NULL DEFAULT 1
        ,   id2   BIGINT
        ,   name  TEXT NOT NULL DEFAULT 'default'
        ,   data  FLOAT(53)
        ,   un1   BIGINT
        ,   un2   BIGINT
        ,   PRIMARY KEY(id, id2)
        );
        CREATE TABLE single_pks
        (   id    BIGSERIAL PRIMARY KEY
        ,   name  TEXT NOT NULL DEFAULT 'default'
        ,   data  FLOAT(53)
        ,   un1  BIGINT
        ,   un2  BIGINT
        );
    ") {
        Ok(_) => println!("created tables\n"),
        Err(e) => panic!("{}", e),
    }

    // --- MultiPk -----------------------------------------------------------

    let mut m = MultiPk {
        id: None,
        id2: 1,
        name: None,
        data: Some(17f32),
        un1: 1,
        un2: 1,
    };
    println!("inserting {:?} .. ", m);
    assert!(m.insert(&mut db).is_ok());
    assert!(format!("{:?}", m) ==
        "MultiPk { id: Some(1), id2: 1, name: Some(\"default\"), \
            data: Some(17.0), un1: 1, un2: 1 }",
    );
    println!("    OK");
    show_multi(&mut db);
    let ms = vec![
        MultiPk {
            id: None,
            id2: 2,
            name: Some(String::from("hej")),
            data: None,
            un1: 1,
            un2: 2,
        },
        MultiPk {
            id: None,
            id2: 3,
            name: Some(String::from("hopp")),
            data: None,
            un1: 1,
            un2: 3,
        },
    ];
    println!("inserting batch {:?} ..", ms);
    let res = MultiPk::insert_batch(&mut db, &ms[..]);
    assert!(res.is_ok());
    assert!(format!("{:?}", res) ==
        "Ok([MultiPk { id: Some(1), id2: 2, name: Some(\"hej\"), \
            data: None, un1: 1, un2: 2 }, \
            MultiPk { id: Some(1), id2: 3, name: Some(\"hopp\"), \
            data: None, un1: 1, un2: 3 }])"
    );
    println!("    OK");
    show_multi(&mut db);
    m = MultiPk {
        id: Some(3),
        id2: 42,
        name: Some(String::from("hej")),
        data: None,
        un1: 1,
        un2: 42,
    };
    println!("not finding non-existing {:?} ..", m);
    let res = m.find_equal(&mut db);
    assert!(res.is_none());
    println!("    OK");
    println!("not finding non-existing by unique fields ..");
    let res = MultiPk::find_by_un1_un2(&mut db, m.un1, m.un2);
    assert!(res.is_none());
    println!("    OK");
    println!("error updating non-existing ..");
    let res = m.update(&mut db);
    assert!(res.is_err());
    println!("    OK");
    println!("inserting non-existing ..");
    let res = m.insert(&mut db);
    assert!(res.is_ok());
    assert!(format!("{:?}", m) ==
        "MultiPk { id: Some(3), id2: 42, name: Some(\"hej\"), \
            data: None, un1: 1, un2: 42 }"
    );
    println!("    OK");
    show_multi(&mut db);
    println!("error inserting existing ..");
    let res = m.insert(&mut db);
    assert!(res.is_err());
    println!("    OK");
    m.name = Some("nytt namn".to_string());
    println!("updating existing {:?} ..", m);
    let res = m.update(&mut db);
    assert!(res.is_ok());
    assert!(format!("{:?}", m) ==
        "MultiPk { id: Some(3), id2: 42, name: Some(\"nytt namn\"), \
            data: None, un1: 1, un2: 42 }"
    );
    println!("    OK");
    show_multi(&mut db);
    println!("finding existing ..");
    let res = m.find_equal(&mut db);
    assert!(res.is_some());
    assert!(format!("{:?}", res.unwrap()) ==
        "MultiPk { id: Some(3), id2: 42, name: Some(\"nytt namn\"), \
            data: None, un1: 1, un2: 42 }"
    );
    println!("    OK");
    println!("finding existing by unique fields ..");
    let res = MultiPk::find_by_un1_un2(&mut db, m.un1, m.un2);
    assert!(res.is_some());
    assert!(format!("{:?}", res.unwrap()) ==
        "MultiPk { id: Some(3), id2: 42, name: Some(\"nytt namn\"), \
            data: None, un1: 1, un2: 42 }"
    );
    println!("    OK");
    println!("deleting existing {:?} ..", m);
    let res = m.clone().delete(&mut db);
    assert!(res.is_ok());
    assert!(res.unwrap() == 1);
    println!("    OK");
    show_multi(&mut db);
    println!("error deleting non-existing {:?}", m);
    let res = m.delete(&mut db);
    assert!(res.is_err());
    println!("    OK");
    for (pks, del) in [
        ([(1, 2), (1, 3)], 0),
        ([(1, 2), (3, 1)], 1),
        ([(2, 1), (1, 1)], 2),
    ].iter() {
        println!("deleting {} out of batch {:?}", del, pks);
        let res = MultiPk::delete_batch(&mut db, pks);
        assert!(res.is_ok());
        assert!(res.unwrap() == *del as u64);
        println!("    OK");
        show_multi(&mut db);
    }

    // --- SinglePk ----------------------------------------------------------

    let mut s = SinglePk {
        id: None,
        name: None,
        data: Some(17f32),
        un1: 1,
        un2: 1,
    };
    println!("inserting {:?} ..", s);
    assert!(s.insert(&mut db).is_ok());
    assert!(format!("{:?}", s) ==
        "SinglePk { id: Some(1), name: Some(\"default\"), data: Some(17.0), \
            un1: 1, un2: 1 }",
    );
    println!("    OK");
    show_single(&mut db);
    let ss = vec![
        SinglePk {
            id: None,
            name: Some(String::from("hej")),
            data: None,
            un1: 1,
            un2: 2,
        },
        SinglePk {
            id: None,
            name: Some(String::from("hopp")),
            data: None,
            un1: 1,
            un2: 3,
        },
    ];
    println!("inserting batch {:?} ..", ss);
    let res = SinglePk::insert_batch(&mut db, &ss[..]);
    assert!(res.is_ok());
    assert!(format!("{:?}", res) ==
        "Ok([SinglePk { id: Some(2), name: Some(\"hej\"), data: None, \
            un1: 1, un2: 2 }, \
            SinglePk { id: Some(3), name: Some(\"hopp\"), data: None, \
            un1: 1, un2: 3 }])"
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
    let res = SinglePk::find_by_un1_un2(&mut db, s.un1, s.un2);
    assert!(res.is_none());
    println!("    OK");
    println!("error updating non-existing ..");
    let res = s.update(&mut db);
    assert!(res.is_err());
    println!("    OK");
    println!("inserting non-existing ..");
    let res = s.insert(&mut db);
    assert!(res.is_ok());
    assert!(format!("{:?}", s) ==
        "SinglePk { id: Some(42), name: Some(\"hej\"), data: None, \
            un1: 1, un2: 42 }"
    );
    println!("    OK");
    show_single(&mut db);
    println!("error inserting existing ..");
    let res = s.insert(&mut db);
    assert!(res.is_err());
    println!("    OK");
    s.name = Some("nytt namn".to_string());
    println!("updating existing {:?} ..", s);
    let res = s.update(&mut db);
    assert!(res.is_ok());
    assert!(format!("{:?}", s) ==
        "SinglePk { id: Some(42), name: Some(\"nytt namn\"), data: None, \
            un1: 1, un2: 42 }"
    );
    println!("    OK");
    show_single(&mut db);
    println!("finding existing ..");
    let res = s.find_equal(&mut db);
    assert!(res.is_some());
    assert!(format!("{:?}", res.unwrap()) ==
        "SinglePk { id: Some(42), name: Some(\"nytt namn\"), data: None, \
            un1: 1, un2: 42 }"
    );
    println!("    OK");
    println!("finding existing by unique fields ..");
    let res = SinglePk::find_by_un1_un2(&mut db, s.un1, s.un2);
    assert!(res.is_some());
    assert!(format!("{:?}", res.unwrap()) ==
        "SinglePk { id: Some(42), name: Some(\"nytt namn\"), data: None, \
            un1: 1, un2: 42 }"
    );
    println!("    OK");
    println!("deleting existing {:?} ..", s);
    let res = s.clone().delete(&mut db);
    assert!(res.is_ok());
    assert!(res.unwrap() == 1);
    println!("    OK");
    show_single(&mut db);
    println!("error deleting non-existing {:?}", s);
    let res = s.delete(&mut db);
    assert!(res.is_err());
    println!("    OK");
    for (pks, del) in [([42, 43], 0), ([42, 3], 1), ([2, 1], 2)].iter() {
        println!("deleting {} out of batch {:?}", del, pks);
        let res = SinglePk::delete_batch(&mut db, pks);
        assert!(res.is_ok());
        assert!(res.unwrap() == *del as u64);
        println!("    OK");
        show_multi(&mut db);
    }
    /*
    for pks in [[42, 43], [42, 3], [2, 1]].iter() {
        println!(
            "deleting batch: {:?} -> {:?}",
             pks,
             SinglePk::delete_batch(&mut db, pks),
        );
        show_single(&mut db);
    }
    */

    // --- The End -----------------------------------------------------------

    db.connection()
        .batch_execute( "
            DROP TABLE multi_pks;
            DROP TABLE single_pks;
        ").unwrap();
}

fn show_multi<'a>(db: &mut impl DbConn<'a>) {
    match SHOW {
        Show::Nothing => (),
        Show::OneLine =>
            println!("--- multi_pks: {:?}", MultiPk::load(db).unwrap()),
        Show::PrettyUgly =>
            println!("--- multi_pks: {:#?}", MultiPk::load(db).unwrap()),
    }
}

fn show_single<'a>(db: &mut impl DbConn<'a>) {
    match SHOW {
        Show::Nothing => (),
        Show::OneLine =>
            println!("--- single_pks: {:?}", SinglePk::load(db).unwrap()),
        Show::PrettyUgly =>
            println!("--- single_pks: {:#?}", SinglePk::load(db).unwrap()),
    }
}

