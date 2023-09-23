use chrono::{NaiveDate, NaiveDateTime};
use vicocomo::{ActiveRecord, DatabaseIf};
pub use {
    default_parent::DefaultParent, multi_pk::MultiPk,
    other_parent::NonstandardParent, single_pk::SinglePk,
};

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
    #[derive(Clone, Debug, vicocomo::ActiveRecord)]
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

    #[derive(Clone, Debug, PartialEq, vicocomo::ActiveRecord)]
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
        pub default_parent_id: Option<i64>,
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
    }
}

pub mod no_pk {
    #[derive(vicocomo::ActiveRecord, Clone, Debug)]
    pub struct NoPk {
        #[vicocomo_order_by(0, "desc")]
        pub data: i32,
    }
}

pub mod other_parent {
    #[derive(Clone, Debug, vicocomo::ActiveRecord)]
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
        #[vicocomo_required]
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
    #[derive(Clone, Debug, vicocomo::ActiveRecord)]
    #[vicocomo_before_delete]
    #[vicocomo_before_save]
    pub struct SinglePk {
        #[vicocomo_optional]
        #[vicocomo_primary]
        pub id: Option<u32>,
        #[vicocomo_order_by(2, "asc")]
        #[vicocomo_optional]
        #[vicocomo_required]
        #[vicocomo_unique = "uni-lbl"]
        pub name: Option<String>,
        #[vicocomo_presence_validator]
        pub data: Option<f32>,
        #[vicocomo_optional]
        pub opt: Option<i32>,
        #[vicocomo_unique = "uni-lbl"]
        #[vicocomo_order_by(1, "desc")]
        #[vicocomo_required]
        pub un2: i32,
    }

    impl vicocomo::BeforeDelete for SinglePk {
        fn before_delete(
            &mut self,
            _db: vicocomo::DatabaseIf,
        ) -> Result<(), vicocomo::Error> {
            if self.name.as_ref().map(|n| n == "immortal").unwrap_or(false) {
                Err(vicocomo::Error::Model(vicocomo::ModelError {
                    error: vicocomo::ModelErrorKind::CannotDelete,
                    model: "SinglePk".to_string(),
                    general: None,
                    field_errors: vec![(
                        "name".to_string(),
                        vec!["Some(\"immortal\")".to_string()],
                    )],
                    assoc_errors: Vec::new(),
                }))
            } else {
                Ok(())
            }
        }
    }

    impl vicocomo::BeforeSave for SinglePk {
        fn before_save(
            &mut self,
            _db: vicocomo::DatabaseIf,
        ) -> Result<(), vicocomo::Error> {
            if self.name.as_ref().map(|n| n.is_empty()).unwrap_or(false) {
                Err(vicocomo::Error::Model(vicocomo::ModelError {
                    error: vicocomo::ModelErrorKind::CannotSave,
                    model: "SinglePk".to_string(),
                    general: None,
                    field_errors: vec![(
                        "name".to_string(),
                        vec!["Some(\"\")".to_string()],
                    )],
                    assoc_errors: Vec::new(),
                }))
            } else {
                Ok(())
            }
        }
    }
}

pub mod view {
    #[derive(vicocomo::ActiveRecord, Clone, Debug)]
    #[vicocomo_readonly]
    pub struct View {
        pub default_parent_id: u32,
        pub count: usize,
    }
}

pub fn empty_db(db: DatabaseIf) {
    let _ = db.clone().exec("DELETE FROM joins", &[]);
    let _ = db.clone().exec("DELETE FROM multi_pks", &[]);
    let _ = db.clone().exec("DELETE FROM single_pks", &[]);
    let _ = db.clone().exec("DELETE FROM default_parents", &[]);
    let _ = db.clone().exec("DELETE FROM nonstandard_parents", &[]);
}

pub fn find_or_insert_default_parent(
    db: DatabaseIf,
    name: &str,
) -> DefaultParent {
    use vicocomo::QueryBld;

    let name = name.to_string();
    let mut found = DefaultParent::query(
        db.clone(),
        &QueryBld::new()
            .col("name")
            .eq(Some(&name.clone().into()))
            .query()
            .unwrap(),
    )
    .unwrap();
    if found.len() == 1 {
        found.drain(..).next().unwrap()
    } else {
        let mut p = DefaultParent { id: None, name };
        assert!(p.insert(db.clone()).is_ok());
        p
    }
}

pub fn find_or_insert_nonstandard_parent(
    db: DatabaseIf,
    name: &str,
) -> NonstandardParent {
    let name = name.to_string();
    if let Some(p) = NonstandardParent::find(db.clone(), &name) {
        p
    } else {
        let mut p = NonstandardParent {
            pk: name,
            nonstandard_parent_id: None,
        };
        assert!(p.insert(db.clone()).is_ok());
        p
    }
}

pub fn find_or_insert_single_pk(
    db: DatabaseIf,
    name: &str,
    un2: i32,
) -> SinglePk {
    let name = name.to_string();
    if let Some(s) = SinglePk::find_by_name_and_un2(db.clone(), &name, &un2) {
        s
    } else {
        let mut s = single_pk::SinglePk {
            id: None,
            name: if name.is_empty() {
                None
            } else {
                Some(name.to_string())
            },
            data: None,
            opt: None,
            un2: un2,
        };
        assert!(s.insert(db.clone()).is_ok());
        s
    }
}

pub fn multi_pk_templ(dp: &DefaultParent) -> MultiPk {
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
        default_parent_id: dp.id,
        other_parent_id: None,
        // NonstandardParent with pk "bonus nonstandard" must not be deleted!
        bonus_parent: "bonus nonstandard".to_string(),
        date_mand: NaiveDate::from_num_days_from_ce_opt(0).unwrap(),
        date_time_mand: NaiveDateTime::from_timestamp_opt(0, 0).unwrap(),
        string_mand: String::new(),
        u32_mand: 0,
        u64_mand: 0,
        usize_mand: 0,
    }
}

pub fn reset_db(
    db: DatabaseIf,
) -> (
    MultiPk,
    MultiPk,
    DefaultParent,
    NonstandardParent,
    NonstandardParent,
) {
    empty_db(db.clone());
    find_or_insert_default_parent(db.clone(), "default filler");
    let dp = find_or_insert_default_parent(db.clone(), "used default");
    let bp = find_or_insert_nonstandard_parent(db.clone(), "bonus nonstandard");
    let np = find_or_insert_nonstandard_parent(db.clone(), "nonstandard");
    let mut m = multi_pk_templ(&dp);
    let mut m2 = m.clone();
    m2.id2 = 2;
    let _ = m.save(db.clone());
    let _ = m2.save(db.clone());
    (m, m2, dp, bp, np)
}

pub fn reset_many_to_many(
    db: DatabaseIf,
) -> (
    default_parent::DefaultParent,
    default_parent::DefaultParent,
    default_parent::DefaultParent,
    single_pk::SinglePk,
    single_pk::SinglePk,
) {
    let (_m, _m2, dp, _bp, _np) = reset_db(db.clone());
    (
        dp,
        find_or_insert_default_parent(db.clone(), "parent-a"),
        find_or_insert_default_parent(db.clone(), "parent-b"),
        find_or_insert_single_pk(db.clone(), "child-a", 101),
        find_or_insert_single_pk(db.clone(), "child-b", 102),
    )
}

pub fn setup(
    db: DatabaseIf,
    auto_primary_sql: &str,
) -> (
    MultiPk,
    MultiPk,
    DefaultParent,
    NonstandardParent,
    NonstandardParent,
) {
    assert!(db.clone().exec("DROP VIEW IF EXISTS views", &[]).is_ok());
    assert!(db.clone().exec("DROP TABLE IF EXISTS joins", &[]).is_ok());
    assert!(db.clone().exec("DROP TABLE IF EXISTS multi_pks", &[]).is_ok());
    assert!(db.clone().exec("DROP TABLE IF EXISTS default_parents", &[]).is_ok());
    assert!(db.clone().exec("DROP TABLE IF EXISTS nonstandard_parents", &[]).is_ok());
    assert!(db.clone().exec("DROP TABLE IF EXISTS no_pks", &[]).is_ok());
    assert!(db.clone().exec("DROP TABLE IF EXISTS single_pks", &[]).is_ok());
    assert!(db.clone()
        .exec(
            &format!(
                "CREATE TABLE default_parents(id {}, name  TEXT NOT NULL)",
                auto_primary_sql,
            ),
            &[],
        )
        .is_ok());
    assert!(db.clone()
        .exec(
            &format!(
                "
                CREATE TABLE single_pks
                (   id    {}
                ,   name  TEXT     NOT NULL DEFAULT 'default'
                ,   data  FLOAT(53)
                ,   opt   BIGINT   DEFAULT 4711
                ,   un2   BIGINT   NOT NULL
                ,   UNIQUE(name, un2)
                )",
                auto_primary_sql,
            ),
            &[],
        )
        .is_ok());
    assert!(db.clone()
        .exec(
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
        .is_ok());
    assert!(db.clone()
        .exec(
            "
            CREATE TABLE nonstandard_parents
            (   pk                     TEXT PRIMARY KEY
            ,   nonstandard_parent_id  TEXT
                    REFERENCES nonstandard_parents ON DELETE RESTRICT
            )",
            &[],
        )
        .is_ok());
    assert!(db.clone()
        .exec(
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
            ,   default_parent_id  BIGINT
                    REFERENCES default_parents ON DELETE CASCADE
            ,   other_parent_id    TEXT
                    REFERENCES nonstandard_parents ON DELETE SET NULL
            ,   bonus_parent      TEXT NOT NULL
                    REFERENCES nonstandard_parents ON DELETE RESTRICT
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
        .is_ok());
    assert!(db.clone()
        .exec("CREATE TABLE no_pks(data BIGINT NOT NULL)", &[])
        .is_ok());
    assert!(db.clone()
        .exec(
            "
            CREATE VIEW views AS
                SELECT default_parent_id, COUNT(single_pk_id) as count
                FROM default_parents JOIN joins
                    ON default_parents.id = joins.default_parent_id
                GROUP BY default_parent_id
            ;",
            &[],
        )
        .is_ok());
    reset_db(db.clone())
}
