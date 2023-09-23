use vicocomo::DatabaseIf;

macro_rules! not_available {
    ($fun:literal, $call:expr) => {
        not_available!($fun, $call, error "not-available")
    };
    ($fun:literal, $call:expr, error $error:literal) => {
        println!("{}() -> Error::Other(\"{}\") ..", $fun, $error);
        let res = $call;
        assert!(res.is_err());
        assert_eq!(res.err().unwrap(), Error::other($error));
        println!("    OK");
    };
    ($fun:literal, $call:expr, None) => {
        println!("{}() -> None ..", $fun);
        let res = $call;
        assert!(res.is_none());
        println!("    OK");
    };
    ($fun:literal, $call:expr, Ok) => {
        println!("{}() -> Ok(_) ..", $fun);
        let res = $call;
        assert!(res.is_ok());
        println!("    OK");
    };
}

pub fn test_no_pk(db: DatabaseIf) {
    use super::models::{no_pk::NoPk, reset_many_to_many, view::View};
    use vicocomo::{ActiveRecord, DbValue, Error, QueryBld};

    super::models::reset_db(db.clone());

    // prepare NoPk

    assert!(db
        .clone()
        .exec("INSERT INTO no_pks(data) VALUES (117), (142);", &[])
        .is_ok());
    let n = NoPk { data: 117 };
    let mut un = n.clone();
    un.data = 118;
    let all_rows_str = "[\
            NoPk { data: 4713 }, \
            NoPk { data: 4712 }, \
            NoPk { data: 4711 }, \
            NoPk { data: 142 }, \
            NoPk { data: 117 }\
        ]";

    // prepare View

    let (_dp, pa, pb, sa, sb) = reset_many_to_many(db.clone());
    assert!(pa.connect_to_single_pk(db.clone(), &sa).is_ok());
    assert!(pa.connect_to_single_pk(db.clone(), &sb).is_ok());
    let v = View::load(db.clone()).unwrap()[0].clone();
    let mut uv = v.clone();
    uv.count = 17;
    let all_views_str = format!(
        "[\
            View {{ \
                default_parent_id: {}, \
                count: 2 \
            }}\
        ]",
        pa.id.unwrap(),
    );

    println!("\nno primary key ------------------------------------------\n");

    println!("inserting - - - - - - - - - - - - - - - - - - - - - - - -\n");

    let mut nn = NoPk { data: 4711 };
    println!("inserting table {:?} ..", nn);
    let old_count = NoPk::load(db.clone()).unwrap().len();
    assert!(nn.insert(db.clone()).is_ok());
    let loaded = NoPk::load(db.clone()).unwrap();
    assert_eq!(loaded.len(), old_count + 1);
    assert!(loaded.iter().find(|n| n.data == 4711).is_some());
    println!("    OK");
    let mut nv = View {
        default_parent_id: pb.id.unwrap() as u32,
        count: 2,
    };
    println!("error inserting view {:?} ..", nv);
    assert!(nv.insert(db.clone()).is_err());
    println!("    OK");
    let mut nb = vec![NoPk { data: 4713 }, NoPk { data: 4712 }];
    println!("inserting table batch {:?} ..", nb);
    let old_count = NoPk::load(db.clone()).unwrap().len();
    assert!(NoPk::insert_batch(db.clone(), &mut nb[..]).is_ok());
    let loaded = NoPk::load(db.clone()).unwrap();
    assert_eq!(loaded.len(), old_count + 2);
    assert_eq!(
        loaded
            .iter()
            .filter(|n| n.data == 4712 || n.data == 4713)
            .count(),
        2,
    );
    println!("    OK");
    let mut vb = vec![
        View {
            default_parent_id: pa.id.unwrap() as u32,
            count: 2,
        },
        View {
            default_parent_id: pb.id.unwrap() as u32,
            count: 2,
        },
    ];
    println!("error inserting view batch {:?} ..", vb);
    assert!(View::insert_batch(db.clone(), &mut vb[..]).is_err());
    println!("    OK");

    println!("\nreading - - - - - - - - - - - - - - - - - - - - - - - - -\n");

    println!("load() ..");
    let found = NoPk::load(db.clone());
    assert!(found.is_ok());
    assert_eq!(&format!("{:?}", &found.unwrap()), &all_rows_str);
    let found = View::load(db.clone());
    assert!(found.is_ok());
    assert_eq!(&format!("{:?}", &found.unwrap()), &all_views_str);
    println!("    OK");
    println!("query() ..");
    let found = NoPk::query(
        db.clone(),
        &QueryBld::new()
            .col("data")
            .lt(Some(&DbValue::Int(4711)))
            .query()
            .unwrap(),
    );
    assert!(found.is_ok());
    let found = found.unwrap();
    assert_eq!(
        &format!("{:?}", found),
        "[\
            NoPk { data: 142 }, \
            NoPk { data: 117 }\
        ]",
    );
    let found = View::query(
        db.clone(),
        &QueryBld::new()
            .col("count")
            .eq(Some(&DbValue::Int(2)))
            .query()
            .unwrap(),
    );
    assert!(found.is_ok());
    let found = found.unwrap();
    assert_eq!(&format!("{:?}", found), &all_views_str);
    println!("    OK");

    println!("\nunavailable functions - - - - - - - - - - - - - - - - - -\n");

    not_available!("table pk_value", n.clone().pk_value(), None);
    not_available!("view pk_value", v.clone().pk_value(), None);
    not_available!("table delete", n.clone().delete(db.clone()));
    not_available!("view delete", v.clone().delete(db.clone()));
    not_available!("table delete_batch", NoPk::delete_batch(db.clone(), &[]));
    not_available!("view delete_batch", View::delete_batch(db.clone(), &[]));
    not_available!("table find", NoPk::find(db.clone(), &()), None);
    not_available!("view find", View::find(db.clone(), &()), None);
    not_available!("table find_equal", n.clone().find_equal(db.clone()), None);
    not_available!("view find_equal", v.clone().find_equal(db.clone()), None);
    not_available!("table update", un.clone().update(db.clone()));
    not_available!("view update", uv.clone().update(db.clone()));
    not_available!(
        "table update_columns",
        n.clone().update_columns(db.clone(), &[("data", DbValue::Int(17))])
    );
    not_available!(
        "view update_columns",
        v.clone()
            .update_columns(db.clone(), &[("default_parent_id", DbValue::Int(17))])
    );
    println!("validate_exists() -> Error::Model(_) ..");
    let res = NoPk::validate_exists(db.clone(), &(), "");
    assert!(res.is_err());
    if let Error::Model(_) = res.err().unwrap() {
        let res = View::validate_exists(db.clone(), &(), "");
        assert!(res.is_err());
        if let Error::Model(_) = res.err().unwrap() {
            println!("    OK");
        } else {
            panic!("expected Error::Model");
        }
    } else {
        panic!("expected Error::Model");
    }
    not_available!("table validate_unique", n.validate_unique(db.clone(), ""), Ok);
    not_available!("view validate_unique", v.validate_unique(db.clone(), ""), Ok);
}
