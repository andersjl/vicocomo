use ::chrono::{NaiveDate, NaiveDateTime};
use ::vicocomo::DatabaseIf;
use {
    default_parent::DefaultParent, multi_pk::MultiPk,
    other_parent::NonstandardParent,
};

pub fn multi_pk_templ() -> MultiPk {
    MultiPk {
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
        // the DefaultParent with id 2 must not be deleted!
        default_parent_id: 2,
        other_parent_id: None,
        // NonstandardParent with pk "bonus nonstandard" must not be deleted!
        bonus_parent: "bonus nonstandard".to_string(),
        date_mand: NaiveDate::from_num_days_from_ce(0),
        date_time_mand: NaiveDateTime::from_timestamp(0, 0),
        string_mand: String::new(),
        u32_mand: 0,
        u64_mand: 0,
        usize_mand: 0,
    }
}

pub fn setup_many_to_many(
    db: DatabaseIf,
) -> (
    default_parent::DefaultParent,
    default_parent::DefaultParent,
    single_pk::SinglePk,
    single_pk::SinglePk,
) {
    use ::vicocomo::Save;
    let mut pa = DefaultParent {
        id: None,
        name: "parent-a".to_string(),
    };
    pa.save(db).unwrap();
    let mut pb = DefaultParent {
        id: None,
        name: "parent-b".to_string(),
    };
    pb.save(db).unwrap();
    let mut sa = single_pk::SinglePk {
        id: None,
        name: Some("child-a".to_string()),
        data: None,
        un1: None,
        un2: 101,
    };
    sa.save(db).unwrap();
    let mut sb = single_pk::SinglePk {
        id: None,
        name: Some("child-b".to_string()),
        data: None,
        un1: None,
        un2: 102,
    };
    sb.save(db).unwrap();
    (pa, pb, sa, sb)
}

// belongs-to associations:
//     MultiPk -> BonusParent
//     MultiPk -> DefaultParent
//     MultiPk -> NonstandardParent
//     NonstandardParent -> NonstandardParent
//
// one-to-many associations:
//     BonusChild        <- MultiPk            restrict
//     DefaultParent     <- MultiPk            cascade
//     NonstandardParent <- MultiPk            forget
//     NonstandardParent <- NonstandardParent  restrict
//
// many-to-many associations:
//     DefaultParent <- joins -> SinglePk

pub mod default_parent {
    #[derive(
        Clone,
        Debug,
        vicocomo::Delete,
        vicocomo::Find,
        vicocomo::HasMany,
        vicocomo::Save,
    )]
    #[vicocomo_has_many(remote_type = "MultiPk", on_delete = "cascade")]
    #[vicocomo_has_many(remote_type = "SinglePk", join_table = "joins")]
    pub struct DefaultParent {
        #[vicocomo_optional]
        #[vicocomo_primary]
        pub id: Option<i64>,
        pub name: String,
    }
}

pub mod multi_pk {
    use chrono::{NaiveDate, NaiveDateTime};

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
        pub date_time_mand: NaiveDateTime,
        pub string_mand: String,
        pub u32_mand: u32,
        pub u64_mand: u64,
        pub usize_mand: usize,
    }

    impl MultiPk {
        pub fn pk(&self) -> String {
            format!(
                "{:?}",
                (
                    self.id,
                    self.id2,
                    self.default_parent_id,
                    &self.other_parent_id,
                    &self.bonus_parent,
                )
            )
        }
        pub fn pks(selves: &Vec<Self>) -> String {
            format!("{:?}", selves.iter().map(|m| m.pk()).collect::<Vec<_>>())
        }
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
        remote_type = "MultiPk"
    )]
    #[vicocomo_has_many(
        name = "BonusChild",
        remote_fk_col = "bonus_parent",
        remote_type = "MultiPk"
    )]
    #[vicocomo_has_many(
        remote_type = "crate::models::other_parent::NonstandardParent"
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

pub mod single_pk {
    #[derive(
        Clone, Debug, ::vicocomo::Delete, ::vicocomo::Find, ::vicocomo::Save,
    )]
    #[vicocomo_before_save]
    pub struct SinglePk {
        #[vicocomo_optional]
        #[vicocomo_primary]
        pub id: Option<u32>,
        #[vicocomo_order_by(2, "asc")]
        #[vicocomo_optional]
        #[vicocomo_unique = "uni-lbl"]
        pub name: Option<String>,
        pub data: Option<f32>,
        #[vicocomo_optional]
        pub un1: Option<i32>,
        #[vicocomo_unique = "uni-lbl"]
        #[vicocomo_order_by(1, "desc")]
        pub un2: i32,
    }

    impl ::vicocomo::BeforeSave for SinglePk {
        fn before_save(
            &mut self,
            _db: ::vicocomo::DatabaseIf,
        ) -> Result<(), ::vicocomo::Error> {
            if self.name.as_ref().map(|n| n.is_empty()).unwrap_or(false) {
                Err(::vicocomo::Error::invalid_input("name empty"))
            } else {
                Ok(())
            }
        }
    }
}

pub fn setup(
    db: DatabaseIf,
) -> (
    MultiPk,
    MultiPk,
    DefaultParent,
    NonstandardParent,
    NonstandardParent,
) {
    use ::vicocomo::{Find, Save};

    db.exec("DROP TABLE IF EXISTS joins", &[]).unwrap();
    db.exec("DROP TABLE IF EXISTS multi_pks", &[]).unwrap();
    db.exec("DROP TABLE IF EXISTS joins", &[]).unwrap();
    db.exec("DROP TABLE IF EXISTS single_pks", &[]).unwrap();
    db.exec("DROP TABLE IF EXISTS default_parents", &[])
        .unwrap();
    db.exec("DROP TABLE IF EXISTS nonstandard_parents", &[])
        .unwrap();
    db.exec(
        "
        CREATE TABLE default_parents
        (   id    BIGSERIAL PRIMARY KEY
        ,   name  TEXT NOT NULL
        )",
        &[],
    )
    .unwrap();
    db.exec(
        "
        CREATE TABLE single_pks
        (   id    BIGSERIAL PRIMARY KEY
        ,   name  TEXT NOT NULL DEFAULT 'default'
        ,   data  FLOAT(53)
        ,   un1   BIGINT DEFAULT 4711
        ,   un2   BIGINT NOT NULL
        ,   UNIQUE(un1, un2)
        )",
        &[],
    )
    .unwrap();
    db.exec(
        "
        CREATE TABLE joins
        (   default_parent_id  BIGINT NOT NULL
                REFERENCES default_parents ON DELETE CASCADE
        ,   single_pk_id       BIGINT NOT NULL
                REFERENCES single_pks ON DELETE CASCADE
        ,   PRIMARY KEY(default_parent_id, single_pk_id)
        )",
        &[],
    )
    .unwrap();
    db.exec(
        "
        CREATE TABLE nonstandard_parents
        (   pk                     TEXT PRIMARY KEY
        ,   nonstandard_parent_id  TEXT
                REFERENCES nonstandard_parents ON DELETE RESTRICT
        )",
        &[],
    )
    .unwrap();
    db.exec(
        "
        CREATE TABLE multi_pks
        (   id                 BIGINT NOT NULL DEFAULT 1
        ,   id2                BIGINT
        ,   bool_mand          BIGINT NOT NULL
        ,   bool_mand_nul      BIGINT
        ,   f32_mand           FLOAT(53) NOT NULL
        ,   f32_opt            FLOAT(53) NOT NULL DEFAULT 1.0
        ,   f64_mand           FLOAT(53) NOT NULL
        ,   f64_opt_nul        FLOAT(53) DEFAULT 1.0
        ,   i32_mand           BIGINT NOT NULL
        ,   i32_opt_nul        BIGINT DEFAULT 1
        ,   default_parent_id  BIGINT NOT NULL
                REFERENCES default_parents ON DELETE CASCADE
        ,   other_parent_id    TEXT
                REFERENCES nonstandard_parents ON DELETE SET NULL
        ,   bonus_parent       TEXT NOT NULL REFERENCES nonstandard_parents
        ,   date_mand          BIGINT NOT NULL
        ,   date_time_mand     BIGINT NOT NULL
        ,   string_mand        TEXT NOT NULL
        ,   u32_mand           BIGINT NOT NULL
        ,   u64_mand           BIGINT NOT NULL
        ,   usize_mand         BIGINT NOT NULL
        ,   PRIMARY KEY(id, id2)
        )",
        &[],
    )
    .unwrap();
    db.exec(
        "
        INSERT INTO default_parents (name)
            VALUES ('default filler'), ('used default')
        ",
        &[],
    )
    .unwrap();
    db.exec(
        "
        INSERT INTO nonstandard_parents (pk, nonstandard_parent_id)
            VALUES ('nonstandard', NULL) , ('bonus nonstandard', NULL)
        ",
        &[],
    )
    .unwrap();
    let dp = DefaultParent::find(db, &2).unwrap();
    let mut m = multi_pk_templ();
    let mut m2 = m.clone();
    m2.id2 = 2;
    m.set_default_parent(&dp).unwrap();
    m2.set_default_parent(&dp).unwrap();
    m.save(db).unwrap();
    m2.save(db).unwrap();
    let bp = NonstandardParent::find(db, &"bonus nonstandard".to_string())
        .unwrap();
    let np = NonstandardParent::find(db, &"nonstandard".to_string()).unwrap();
    (m, m2, dp, bp, np)
}
