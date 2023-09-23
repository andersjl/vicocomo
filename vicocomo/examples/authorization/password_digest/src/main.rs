use ::vicocomo::{is_error, ActiveRecord, Error, PasswordDigest};
use ::vicocomo_bcrypt::{BcryptDigest, OptBcryptDigest};

#[derive(ActiveRecord)]
struct MyUser {
    #[vicocomo_primary]
    name: String,
    // inform ActiveRecord that OptBcryptDigest can be converted to and
    // from DbValue::NulText
    #[vicocomo_db_value = "NulText"]
    password_digest: OptBcryptDigest,
}

impl MyUser {
    fn set(
        &mut self,
        password: Option<&str>,
        pwd_conf: &str,
        valid: Option<&dyn Fn(&str) -> Result<(), Error>>,
    ) -> Result<(), Error> {
        self.password_digest = OptBcryptDigest(match password {
            Some(pwd) => Some(PasswordDigest::set(
                pwd,
                pwd_conf,
                valid.unwrap_or(&(|_| Ok(()))),
            )?),
            None => None,
        });
        Ok(())
    }

    fn authenticate(&self, password: &str) -> bool {
        self.password_digest
            .0
            .as_ref()
            .map(|digest| digest.authenticate(password))
            .unwrap_or(false)
    }
}

fn main() {
    BcryptDigest::set_cost(4);
    let mut user = MyUser {
        name: "Some Name".to_string(),
        password_digest: OptBcryptDigest(None),
    };
    println!("setting password ..");
    let res = user.set(Some("password"), "not-confirmed", None);
    assert!(res.is_err());
    assert!(is_error!(
        res.err().unwrap(),
        InvalidInput("password--differ")
    ));
    assert!(user.set(Some("password"), "password", None).is_ok());
    println!("    OK");
    println!("authenticating ..");
    assert!(user.authenticate("password"));
    println!("    OK");
    println!("removing password ..");
    assert!(user.set(None, "", None).is_ok());
    assert_eq!(user.password_digest, OptBcryptDigest(None));
    assert!(!user.authenticate("password"));
    assert!(!user.authenticate(""));
    println!("    OK");
    println!("validation ..");
    assert!(user
        .set(
            Some("password"),
            "password",
            Some(&|pwd| {
                if pwd.len() < 8 {
                    return Err(Error::other("my-error"));
                }
                Ok(())
            }),
        )
        .is_ok());
    let res = user.set(
        Some("password"),
        "password",
        Some(&|_| Err(Error::other("my-error"))),
    );
    assert!(res.is_err());
    assert!(is_error!(res.err().unwrap(), Other("my-error")));
    println!("    OK");
}
