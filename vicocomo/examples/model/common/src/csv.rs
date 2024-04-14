use vicocomo::DatabaseIf;
pub fn test_csv(db: DatabaseIf) {
    use super::models::{
        backup, restore, DefaultParent, Join, MultiPk, NoPk,
        NonstandardParent, Random, SinglePk,
    };
    use std::str::from_utf8;
    use vicocomo::{check_backup, ActiveRecord, Error};

    super::models::reset_db(db.clone());

    println!("\nCSV files -----------------------------------------------\n");

    assert_eq!(
        db.clone().exec(
            "INSERT INTO single_pks (id, name, data, un2) VALUES \
            (4711, '''foo''\n\"bar\";baz', NULL, 1)",
            &[],
        ),
        Ok(1),
    );

    println!("save and load one table to/from CSV - - - - - - - - - - -\n");

    println!("saving table as CSV ..");
    let csv = SinglePk::try_to_csv(db.clone(), None);
    assert!(csv.is_ok());
    let (_, csv) = csv.unwrap();
    assert_eq!(
        csv,
        "id,name,data,opt,un2\r\n\
        4711,\"'foo'\n\"\"bar\"\";baz\",,4711,1\r\n",
    );
    assert_eq!(
        db.clone().exec(
            "INSERT INTO single_pks (id, name, data, opt, un2) VALUES \
            (4710, 'two', +.07e-3, 42, 1)",
            &[],
        ),
        Ok(1),
    );
    let csv = SinglePk::try_to_csv(db.clone(), Some(b';'));
    assert!(csv.is_ok());
    let (table, csv) = csv.unwrap();
    assert_eq!(table, "single_pks");
    assert!(csv.contains("4710;\"two\";"));
    assert!(csv.contains(";42;1"));

    println!("loading table from saved CSV ..");
    let old = SinglePk::load(db.clone()).unwrap();
    assert!(SinglePk::try_from_csv(db.clone(), &csv, Some(b';')).is_ok());
    assert_eq!(SinglePk::load(db.clone()).unwrap(), old);
    assert!(db.clone().exec("DELETE FROM multi_pks", &[]).is_ok());
    assert!(SinglePk::try_from_csv(db.clone(), &csv, Some(b';')).is_ok());
    assert_eq!(SinglePk::load(db.clone()).unwrap(), old);
    println!("    OK");

    println!("\nimport from CSV - - - - - - - - - - - - - - - - - - - - -\n");

    println!("error when to many columns in CSV value line ..");
    let result = SinglePk::try_from_csv(
        db.clone(),
        "id,un2,data\n1,2,\n4,5,6,\r\n7,8,9\n",
        None,
    );
    assert_eq!(
        result,
        Err(Error::invalid_input(
            r#"invalid-csv: [Some("4"), Some("5"), Some("6"), None]"#,
        )),
    );
    println!("    OK");

    println!("error when importing NULL optional required fields ..");
    let result = SinglePk::try_from_csv(
        db.clone(),
        "id,\"un2\",data,name\n1,2,3,\n4,5,6,\"a nam\"\r\n7,8,9,\"nother\"\n",
        None,
    );
    match result {
        Err(Error::InvalidInput(_)) => (),
        _ => panic!("expected Error::InvalidInput, got {:?}", result),
    }
    println!("    OK");

    println!("loading table from imported CSV ..");
    let result = SinglePk::try_from_csv(
        db.clone(),
        "id,\"un2\",data,opt,name\n1,2,\"2.5\",3,a name\n4,5,6,7,another\
        \r\n8,9,9.5,10,\"yet another\"\n",
        None,
    );
    assert!(result.is_ok(), "{:?}", result);
    let result = SinglePk::load(db.clone());
    assert!(result.is_ok(), "{:?}", result);
    assert_eq!(
        SinglePk::load(db.clone()).unwrap(),
        vec![
            SinglePk {
                id: Some(8),
                name: Some(String::from("yet another")),
                data: Some(9.5),
                opt: Some(10),
                un2: 9,
            },
            SinglePk {
                id: Some(4),
                name: Some(String::from("another")),
                data: Some(6.0),
                opt: Some(7),
                un2: 5,
            },
            SinglePk {
                id: Some(1),
                name: Some(String::from("a name")),
                data: Some(2.5),
                opt: Some(3),
                un2: 2,
            },
        ],
    );
    println!("    OK");

    println!("\nbackup and restore to/from CSV  - - - - - - - - - - - - -\n");

    super::models::reset_db(db.clone());

    println!("check_backup()");
    assert!(check_backup(b"short").is_err());
    assert!(check_backup(&ver_bytes("0", "5")).is_err());
    let curver = ver_bytes("0", "6");
    let res = check_backup(&curver);
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), b"");
    let mut binver = curver.clone();
    binver.extend(b"b\xffnary");
    let res = check_backup(&binver);
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), b"b\xffnary");
    println!("    OK");
    println!("backup to CSV ..");
    let bkp = backup(db.clone());
    assert!(bkp.is_ok(), "{bkp:?}");
    let bkp = bkp.unwrap();
    let contents = check_backup(&bkp);
    assert!(contents.is_ok(), "{:?}", contents);
    let contents = &contents.unwrap();
    let bkp_str = from_utf8(contents);
    assert!(
        bkp_str.is_ok(),
        "{}",
        String::from_utf8_lossy(&contents[..50])
    );
    let bkp_str = bkp_str;
    assert_eq!(
        bkp_str.unwrap(),
        "--- joins ---\r\n\
        \r\n\
        --- default_parents ---\r\n\
        id,name\r\n\
        33,\"default filler\"\r\n\
        34,\"used default\"\r\n\
        --- multi_pks ---\r\n\
        id,id2,bool_mand,bool_mand_nul,f32_mand,f32_opt,f64_mand,f64_opt_nul,\
        i32_mand,i32_opt_nul,default_parent_id,other_parent_id,bonus_parent,\
        date_mand,date_time_mand,string_mand,u32_mand,u64_mand,usize_mand\r\n\
        1,1,0,,0,1,0,1,0,1,34,,\"bonus nonstandard\",0,0,\"\",0,0,0\r\n\
        1,2,0,,0,1,0,1,0,1,34,,\"bonus nonstandard\",0,0,\"\",0,0,0\r\n\
        --- no_pks ---\r\n\
        data\r\n\
        4713\r\n\
        4712\r\n\
        4711\r\n\
        142\r\n\
        117\r\n\
        --- nonstandard_parents ---\r\n\
        pk,nonstandard_parent_id\r\n\
        \"bonus nonstandard\",\r\n\
        \"nonstandard\",\r\n\
        --- randoms ---\r\n\
        \r\n\
        --- single_pks ---\r\n\
        \r\n",
    );
    println!("    OK");

    println!("restore from CSV ..");
    let joins = Join::load(db.clone()).unwrap();
    let default_parents = DefaultParent::load(db.clone()).unwrap();
    let multi_pks = MultiPk::load(db.clone()).unwrap();
    let no_pks = NoPk::load(db.clone()).unwrap();
    let nonstandard_parents = NonstandardParent::load(db.clone()).unwrap();
    let randoms = Random::load(db.clone()).unwrap();
    let single_pks = SinglePk::load(db.clone()).unwrap();
    let res = restore(db.clone(), &bkp);
    assert!(res.is_ok(), "{res:?}");
    assert_eq!(Join::load(db.clone()).unwrap(), joins);
    assert_eq!(DefaultParent::load(db.clone()).unwrap(), default_parents);
    assert_eq!(MultiPk::load(db.clone()).unwrap(), multi_pks);
    assert_eq!(NoPk::load(db.clone()).unwrap(), no_pks);
    assert_eq!(
        NonstandardParent::load(db.clone()).unwrap(),
        nonstandard_parents,
    );
    assert_eq!(Random::load(db.clone()).unwrap(), randoms);
    assert_eq!(SinglePk::load(db.clone()).unwrap(), single_pks);
    println!("    OK");
    println!("restore from CSV with models shuffled ..");
    let mut shuffled = curver.clone();
    shuffled.extend(
        "--- multi_pks ---\r\n\
        id,id2,bool_mand,bool_mand_nul,f32_mand,f32_opt,f64_mand,f64_opt_nul,\
        i32_mand,i32_opt_nul,default_parent_id,other_parent_id,bonus_parent,\
        date_mand,date_time_mand,string_mand,u32_mand,u64_mand,usize_mand\r\n\
        1,1,0,,0,1,0,1,0,1,34,,\"bonus nonstandard\",0,0,\"\",0,0,0\r\n\
        1,2,0,,0,1,0,1,0,1,34,,\"bonus nonstandard\",0,0,\"\",0,0,0\r\n\
        --- single_pks ---\r\n\
        \r\n\
        --- nonstandard_parents ---\r\n\
        pk,nonstandard_parent_id\r\n\
        \"bonus nonstandard\",\r\n\
        \"nonstandard\",\r\n\
        --- randoms ---\r\n\
        \r\n\
        --- default_parents ---\r\n\
        id,name\r\n\
        33,\"default filler\"\r\n\
        34,\"used default\"\r\n\
        --- joins ---\r\n\
        \r\n\
        --- no_pks ---\r\n\
        data\r\n\
        4713\r\n\
        4712\r\n\
        4711\r\n\
        142\r\n\
        117\r\n"
            .as_bytes(),
    );
    let res = restore(db.clone(), &shuffled);
    assert!(res.is_ok(), "{res:?}");
    assert_eq!(Join::load(db.clone()).unwrap(), joins);
    assert_eq!(DefaultParent::load(db.clone()).unwrap(), default_parents);
    assert_eq!(MultiPk::load(db.clone()).unwrap(), multi_pks);
    assert_eq!(NoPk::load(db.clone()).unwrap(), no_pks);
    assert_eq!(
        NonstandardParent::load(db.clone()).unwrap(),
        nonstandard_parents,
    );
    assert_eq!(Random::load(db.clone()).unwrap(), randoms);
    assert_eq!(SinglePk::load(db.clone()).unwrap(), single_pks);
    println!("    OK");
}

fn ver_bytes(maj: &str, min: &str) -> Vec<u8> {
    format!("--- vicocomo backup format version {maj}.{min} ---\r\n")
        .into_bytes()
}
