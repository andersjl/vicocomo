//! Implement `vicocomo::PasswordDigest` by way of the [`bcrypt`
//! ](../bcrypt/index.html) crate.

use bcrypt::{hash, verify};
use std::sync::{Mutex, OnceLock};
use vicocomo::{db_value_convert, map_error, Error, PasswordDigest};

/// Encapsulates a [BCrypt](../bcrypt/index.html) hash.
///
/// Defines conversions to and from
/// [`DbValue::Text`](../vicocomo/database/enum.DbValue.html#variant.Text) and
/// a type `OptBcryptDigest = Option<BcryptDigest>` with conversions to and
/// from [`DbValue::NulText`
/// ](../vicocomo/database/enum.DbValue.html#variant.NulText).
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BcryptDigest(String);

static BCRYPT_COST: OnceLock<Mutex<u32>> = OnceLock::new();

impl BcryptDigest {
    /// Set the BCrypt [cost](https://github.com/Keats/rust-bcrypt#readme).
    ///
    pub fn set_cost(cost: u32) {
        *BCRYPT_COST.get_or_init(|| Mutex::new(0)).lock().unwrap() = cost;
    }
}

impl PasswordDigest for BcryptDigest {
    /// The [cost](https://github.com/Keats/rust-bcrypt#readme) is as set in
    /// the environment variable `BCRYPT_COST` or [`bcrypt::DEFAULT_COST`
    /// ](../bcrypt/constant.DEFAULT_COST.html).
    ///
    fn digest(password: &str) -> Result<Self, Error> {
        Ok(Self(map_error!(
            Other,
            hash(
                password,
                *BCRYPT_COST
                    .get_or_init(|| {
                        Mutex::new({
                            let cost: u32 = std::env::var("BCRYPT_COST")
                                .unwrap_or(String::new())
                                .parse()
                                .unwrap_or(bcrypt::DEFAULT_COST);
                            if cost != bcrypt::DEFAULT_COST {
                                eprintln!("using bcrypt cost {}", cost);
                            }
                            cost
                        })
                    })
                    .lock()
                    .unwrap(),
            ),
        )?))
    }

    /// Wraps [`bcrypt::verify()`](../bcrypt/fn.verify.html) turning errors to
    /// `false`.
    ///
    fn authenticate(&self, password: &str) -> bool {
        match verify(password, &self.0) {
            Ok(result) => result,
            Err(_) => false,
        }
    }
}

use vicocomo::DbValue;
db_value_convert! {
    BcryptDigest,
    Text,
    BcryptDigest(value),
    other.0
}

#[test]
fn vicocomo_bcrypt_test() {
    use vicocomo::is_error;
    std::env::set_var("BCRYPT_COST", "4");
    let password = "hej";
    let digest = BcryptDigest::digest(password);
    assert!(digest.is_ok());
    let digest = digest.unwrap();
    assert_ne!(digest.0, password);
    assert!(digest.authenticate(password));
    assert!(!digest.authenticate("nej"));
    BcryptDigest::set_cost(0);
    let digest = BcryptDigest::digest(password);
    assert!(digest.is_err());
    assert!(is_error!(digest.err().unwrap(), Other));
}
