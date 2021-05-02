use ::chrono::NaiveDate;
use ::serde::{Deserialize, Serialize};
use ::serde_json::json;
use ::vicocomo::{
    cannot_save, HtmlForm, HtmlInput, HttpServerIf, InputType, SessionModel,
};
use ::vicocomo_stubs::HttpServerStub;

#[derive(Clone, Debug, HtmlForm)]
struct BigForm {
    errors: Vec<String>,
    num: HtmlInput<u32>,
    lin: HtmlInput<String>,
    #[vicocomo_html_input_type = "Textarea"]
    txt: HtmlInput<String>,
    #[vicocomo_html_input_type = "Hidden"]
    hid: HtmlInput<String>,
    dat: HtmlInput<NaiveDate>,
    #[vicocomo_html_input_type = "Select"]
    sel: HtmlInput<u32>,
    #[vicocomo_html_input_type = "Radio"]
    rad: HtmlInput<i32>,
    #[vicocomo_html_input_type = "SelectMult"]
    mul: HtmlInput<i64>,
    #[vicocomo_html_input_type = "Checkbox"]
    chk: HtmlInput<String>,
    foo: Option<String>,
}

#[derive(Clone, Debug, HtmlForm)]
struct SmallForm {
    errors: Vec<String>,
    num: HtmlInput<u32>,
    #[vicocomo_html_input_type = "Radio"]
    rad: HtmlInput<i32>,
    #[vicocomo_html_input_type = "SelectMult"]
    mul: HtmlInput<i64>,
    foo: Option<String>,
}

#[derive(Clone, Debug, Deserialize, HtmlForm, Serialize, SessionModel)]
struct SessionForm {
    errors: Vec<String>,
    num: HtmlInput<u32>,
    #[vicocomo_html_input_type = "Radio"]
    rad: HtmlInput<i32>,
    #[vicocomo_html_input_type = "SelectMult"]
    mul: HtmlInput<i64>,
    foo: Option<String>,
}

fn main() {
    print!("InputType::Date ... ");

    let mut date: HtmlInput<NaiveDate> =
        HtmlInput::new(InputType::Date, "dat");
    date.set_attr("id", Some("dat-id"));
    date.set("2020-02-02".parse().unwrap());
    date.add_attr_vals("class", " a");
    date.add_attr_vals("class", "b c ");
    date.add_attr_vals("class", "d \n");
    assert_eq!(
        date.render(),
        json!({
            "errors":[],
            "data": {
                "label": null,
                "tag": concat!(
                    r#"<input type="date" id="dat-id" name="dat""#,
                    r#" value="2020-02-02" class="a b c d">"#,
                ),
            },
        }),
    );
    date.set_label("some-label");
    assert_eq!(
        date.render(),
        json!({
            "errors": [],
            "data": {
                "label": r#"<label for="dat-id">some-label</label>"#,
                "tag": concat!(
                    r#"<input type="date" id="dat-id" name="dat""#,
                    r#" value="2020-02-02" class="a b c d">"#,
                ),
            },
        }),
    );
    println!("OK");

    print!("InputType::Checkbox ... ");

    let mut checkboxes: HtmlInput<u32> =
        HtmlInput::new(InputType::Checkbox, "chk");
    checkboxes.set_attr("id", Some("chk-id"));
    checkboxes.set_options(&[("answer", 42), ("less", 1), ("sweet", 16)]);
    checkboxes.set_mult(&[16, 17, 42]);
    checkboxes.set_attr("class", Some("be bop"));
    checkboxes.set_label("");
    assert_eq!(
        checkboxes.render(),
        json!({
        "errors":[],
        "data": [
            {
                "label": r#"<label for="chk-id--42">answer</label>"#,
                "tag": concat!(
                    r#"<input type="checkbox" id="chk-id--42""#,
                    r#" name="chk[]" class="be bop" value="42" checked>"#,
                ),
            }, {
                "label": r#"<label for="chk-id--1">less</label>"#,
                "tag": concat!(
                    r#"<input type="checkbox" id="chk-id--1" name="chk[]""#,
                    r#" class="be bop" value="1">"#,
                ),
            }, {
                "label": r#"<label for="chk-id--16">sweet</label>"#,
                "tag": concat!(
                    r#"<input type="checkbox" id="chk-id--16" name="chk[]""#,
                    r#" class="be bop" value="16" checked>"#,
                ),
            },
        ],
                }),
    );
    println!("OK");

    print!("InputType::Radio ... ");

    let mut radiobtns: HtmlInput<isize> =
        HtmlInput::new(InputType::Radio, "rad");
    radiobtns.set_attr("id", Some("rad-id"));
    radiobtns.set_options(&[("answer", -42), ("sweet", 16)]);
    radiobtns.set(-42);
    radiobtns.set_attr("class", Some("be bop"));
    assert_eq!(
        radiobtns.render(),
        json!({
        "errors":[],
        "data": [
            {
                "label": null,
                "tag": concat!(
                    r#"<input type="radio" id="rad-id---42" name="rad""#,
                    r#" class="be bop" value="-42" checked>"#,
                ),
            }, {
                "label": null,
                "tag": concat!(
                    r#"<input type="radio" id="rad-id--16" name="rad""#,
                    r#" class="be bop" value="16">"#,
                ),
            },
        ],
                }),
    );
    println!("OK");

    print!("InputType::Select ... ");

    let mut select: HtmlInput<String> =
        HtmlInput::new(InputType::Select, "sel");
    select.set_attr("id", Some("sel-id"));
    select.set_options(&[
        ("answer", "42".to_string()),
        ("less", "1".to_string()),
        ("sweet", "16".to_string()),
    ]);
    select.set("16".to_string());
    select.set_attr("class", Some("be bop"));
    assert_eq!(
        select.render(),
        json!({
            "errors":[],
            "data": {
                "label": null,
                "tag": concat!(
                    r#"<select id="sel-id" name="sel" class="be bop">"#,
                    r#"<option value="42">answer</option>"#,
                    r#"<option value="1">less</option>"#,
                    r#"<option value="16" selected>sweet</option>"#,
                    r#"</select>"#,
                ),
            },
        }),
    );
    println!("OK");

    print!("InputType::SelectMult ... ");

    let mut multiple: HtmlInput<String> =
        HtmlInput::new(InputType::SelectMult, "mul");
    multiple.set_attr("id", Some("mul-id"));
    multiple.set_options(&[
        ("answer", "42".to_string()),
        ("less", "1".to_string()),
        ("sweet", "16".to_string()),
    ]);
    multiple.set_mult(&["16".to_string(), "1".to_string()]);
    multiple.set_attr("class", Some("be bop"));
    multiple.set_label("some-label");
    assert_eq!(
        multiple.render(),
        json!({
            "errors":[],
            "data": {
                "label": r#"<label for="mul-id">some-label</label>"#,
                "tag": concat!(
                    r#"<select multiple id="mul-id" name="mul" class="be bop">"#,
                    r#"<option value="42">answer</option>"#,
                    r#"<option value="1" selected>less</option>"#,
                    r#"<option value="16" selected>sweet</option>"#,
                    r#"</select>"#,
                ),
            },
        }),
    );
    println!("OK");

    print!("InputType::Textarea ... ");

    let mut area: HtmlInput<String> =
        HtmlInput::new(InputType::Textarea, "txt");
    area.set_attr("id", Some("txt-id"));
    area.set("text".to_string());
    area.add_attr_vals("class", "be bop");
    assert_eq!(
        area.render(),
        json!({
            "errors":[],
            "data": {
                "label": null,
                "tag": concat!(
                    r#"<textarea id="txt-id" name="txt" class="be bop">"#,
                    r#"text"#,
                    r#"</textarea>"#,
                ),
            },
        }),
    );
    area.set_label("some-label");
    assert_eq!(
        area.render(),
        json!({
            "errors":[],
            "data": {
                "label": r#"<label for="txt-id">some-label</label>"#,
                "tag": concat!(
                    r#"<textarea id="txt-id" name="txt" class="be bop">"#,
                    r#"text"#,
                    r#"</textarea>"#,
                ),
            },
        }),
    );
    println!("OK");

    print!("BigForm update ... ");

    let mut big = BigForm::new();

    // numeric field
    assert!(big.num.get().is_none());
    big.num.set(17u32);
    assert!(big.num.get().is_some());
    assert_eq!(big.num.get().unwrap(), 17u32);
    assert!(big.update(&json!({"num": "42"})).is_ok());
    assert_eq!(big.num.get().unwrap(), 42u32);
    // missing field in JSON is not updated
    assert!(big.update(&json!({})).is_ok());
    assert_eq!(big.num.get().unwrap(), 42u32);
    // unrecognized key in JSON is ignored
    assert!(big.update(&json!({"num": "43", "foo": "bar"})).is_ok());
    assert_eq!(big.num.get().unwrap(), 43u32);

    // CheckBox, Radio, Select, and SelectMult need an (option, value) list
    big.sel.set_options(&[("a", 1), ("b", 42)]);
    big.rad.set_options(&[("a", 1), ("b", -42)]);
    big.mul.set_options(&[("a", 1), ("b", ::std::i64::MIN)]);
    big.chk
        .set_options(&[("a", "x".to_string()), ("b", "y".to_string())]);
    // setting a value to something not among the options => None / empty
    big.sel.set(17);
    assert!(big.sel.get().is_none());
    big.mul.set_mult(&[17, 42]);
    assert!(big.sel.get_mult().is_empty());
    // single select field
    big.rad.set(-42);
    assert!(big.rad.get().is_some());
    assert_eq!(big.rad.get().unwrap(), -42);
    // set_options() keeps set option that is also in the new list - single
    big.rad.set_options(&[("c", 17), ("d", -42)]);
    assert!(big.rad.get().is_some());
    assert_eq!(big.rad.get().unwrap(), -42); // note that it is "renamed"!
                                             // remove set option from single select
    big.rad.set_options(&[("e", 1), ("f", 42)]);
    assert!(big.rad.get().is_none());
    big.rad.set_options(&[("c", 17), ("d", -42)]);
    // remove / keep set option from multiple select
    big.chk.set_mult(&["y".to_string(), "x".to_string()]);
    big.chk
        .set_options(&[("c", "z".to_string()), ("d", "y".to_string())]);
    assert_eq!(big.chk.get_mult().len(), 1);
    assert_eq!(big.chk.get_mult().first().unwrap(), "y");

    assert!(big
        .update(&json!({
            "num": "17",
            "lin": "line",
            "txt": "text",
            "hid": "hidden",
            "dat": "2020-02-02",
            "sel": "42",
            "rad": "-42",
            "mul": ["-9223372036854775808", "1"],
            "chk": ["z"],
        }))
        .is_ok(),);

    assert!(big.foo.is_none());

    assert_eq!(
        big.to_json(),
        json!({
        "chk": {
            "errors": [],
            "data": [
        {   "label": null,
            "tag": r#"<input type="checkbox" id="chk--z" name="chk[]" value="z" checked>"#,
        },{ "label": null,
            "tag": r#"<input type="checkbox" id="chk--y" name="chk[]" value="y">"#,
        }]},
        "dat": {
            "errors": [],
            "data":
        {   "label": null,
            "tag": r#"<input type="date" id="dat" name="dat" value="2020-02-02">"#,
        }},
        "foo": null,
        "hid": {
            "errors": [],
            "data":
        {   "label": null,
            "tag": r#"<input type="hidden" id="hid" name="hid" value="hidden">"#,
        }},
        "lin": {
            "errors": [],
            "data":
        {   "label": null,
            "tag": r#"<input type="text" id="lin" name="lin" value="line">"#,
        }},
        "mul": {
            "errors": [],
            "data":
        {   "label": null,
            "tag": concat!(
                r#"<select multiple id="mul" name="mul">"#,
                r#"<option value="1" selected>a</option>"#,
                r#"<option value="-9223372036854775808" selected>b</option>"#,
                r#"</select>"#,
        )}},
        "num": {
            "errors": [],
            "data":
        {   "label": null,
            "tag": r#"<input type="number" id="num" name="num" value="17">"#,
        }},
        "rad": {
            "errors": [],
            "data": [
        {   "label": null,
            "tag": r#"<input type="radio" id="rad--17" name="rad" value="17">"#,
        },{ "label": null,
            "tag": r#"<input type="radio" id="rad---42" name="rad" value="-42" checked>"#,
        }]},
        "sel": {
            "errors": [],
            "data":
        {   "label": null,
            "tag": concat!(
                r#"<select id="sel" name="sel">"#,
                r#"<option value="1">a</option>"#,
                r#"<option value="42" selected>b</option>"#,
                r#"</select>"#,
        )}},
        "txt": {
            "errors": [],
            "data":
        {   "label": null,
            "tag": r#"<textarea id="txt" name="txt">text</textarea>"#,
        }},
                }),
    );
    println!("OK");

    print!("SmallForm update ... ");

    let mut small = SmallForm::new();

    // Radio and SelectMult need an (option, value) list
    small.rad.set_options(&[("a", 1), ("b", -42)]);
    small.mul.set_options(&[("p", 1), ("n", 0), ("m", -1)]);

    assert!(small
        .update(&json!({
            "num": "17",
            "rad": "-42",
            "mul": ["-1", "1"],
        }))
        .is_ok(),);

    let expected_small_json = json!({
    "foo": null,
    "mul": {
        "errors": [],
        "data":
    {   "label": null,
        "tag": concat!(
            r#"<select multiple id="mul" name="mul">"#,
            r#"<option value="1" selected>p</option>"#,
            r#"<option value="0">n</option>"#,
            r#"<option value="-1" selected>m</option>"#,
            r#"</select>"#,
        ),
    }},
    "num": {
        "errors": [],
        "data":
    {   "label": null,
        "tag": r#"<input type="number" id="num" name="num" value="17">"#,
    }},
    "rad": {
        "errors": [],
        "data": [
    {   "label": null,
        "tag": r#"<input type="radio" id="rad--1" name="rad" value="1">"#,
    },{ "label": null,
        "tag": r#"<input type="radio" id="rad---42" name="rad" value="-42" checked>"#,
    }]},
        });

    assert_eq!(small.to_json(), expected_small_json);
    println!("OK");

    print!("SmallForm with labels and prefix ... ");

    let mut labels = SmallForm::with_labels(Some("prefix"));
    labels.rad.set_options(&[("a", 1), ("b", -42)]);
    labels.mul.set_options(&[("p", 1), ("n", 0), ("m", -1)]);
    assert!(labels
        .update(&json!({
            "num": "17",
            "rad": "-42",
            "mul": ["-1", "1"],
        }))
        .is_ok(),);

    let labels_json = json!({
    "foo": null,
    "mul": {
        "errors": [],
        "data":
    {   "label": r#"<label for="mul">prefix--SmallForm--mul--label</label>"#,
        "tag": concat!(
            r#"<select multiple id="mul" name="mul">"#,
                r#"<option value="1" selected>p</option>"#,
                r#"<option value="0">n</option>"#,
                r#"<option value="-1" selected>m</option>"#,
            r#"</select>"#,
        ),
    }},
    "num": {
        "errors": [],
        "data":
    {   "label": r#"<label for="num">prefix--SmallForm--num--label</label>"#,
        "tag": r#"<input type="number" id="num" name="num" value="17">"#,
    }},
    "rad": {
        "errors": [],
        "data": [
    {   "label": r#"<label for="rad--1">a</label>"#,
        "tag": r#"<input type="radio" id="rad--1" name="rad" value="1">"#,
    },{ "label": r#"<label for="rad---42">b</label>"#,
        "tag": r#"<input type="radio" id="rad---42" name="rad" value="-42" checked>"#,
    }]}
        });

    assert_eq!(labels.to_json(), labels_json);
    println!("OK");

    print!("SmallForm with labels, no prefix ... ");

    let mut nopre = SmallForm::with_labels(None);
    nopre.rad.set_options(&[("", 1), ("", -42)]);
    nopre.rad.clear_label();
    nopre.mul.set_options(&[("p", 1), ("n", 0)]);
    let before = nopre.to_json_values();
    assert!(nopre
        .update(&json!({
            "num": "17",
            "rad": "-42",
            "mul": ["-1", "1"],  // the -1 is not an option
        }))
        .is_err(),);
    assert_eq!(nopre.to_json_values(), before);
    nopre.clear_errors();
    assert!(nopre
        .update(&json!({
            "num": "17",
            "rad": "-42",
            "mul": ["1"],
        }))
        .is_ok(),);

    let nopre_json = json!({
    "foo": null,
    "mul": {
        "errors": [],
        "data":
    {   "label": r#"<label for="mul">SmallForm--mul--label</label>"#,
        "tag": concat!(
            r#"<select multiple id="mul" name="mul">"#,
            r#"<option value="1" selected>p</option>"#,
            r#"<option value="0">n</option>"#,
            r#"</select>"#,
        ),
    }},
    "num": {
        "errors": [],
        "data":
    {   "label": r#"<label for="num">SmallForm--num--label</label>"#,
        "tag": r#"<input type="number" id="num" name="num" value="17">"#,
    }},
    "rad": {
        "errors": [],
        "data": [
    {   "label": null,
        "tag": r#"<input type="radio" id="rad--1" name="rad" value="1">"#,
    },{ "label": null,
        "tag": r#"<input type="radio" id="rad---42" name="rad" value="-42" checked>"#,
    }]},
        });

    assert_eq!(nopre.to_json(), nopre_json);
    println!("OK");

    print!("SessionForm to_json_session() ... ");

    let server_stub = HttpServerStub::new();
    let server = HttpServerIf::new(&server_stub);

    let mut sess_form = SessionForm::new();

    // Radio and SelectMult need an (option, value) list
    sess_form.rad.set_options(&[("a", 1), ("b", -42)]);
    sess_form.mul.set_options(&[("p", 1), ("n", 0), ("m", -1)]);

    assert!(sess_form
        .update(&json!({
            "num": "17",
            "rad": "-42",
            "mul": ["-1", "1"],
        }))
        .is_ok(),);

    assert_eq!(
        sess_form.to_json_session(server).unwrap(),
        expected_small_json
    );
    println!("OK");

    print!("SessionForm update_session(), no input ... ");
    let old_session = sess_form.clone();
    assert!(server.session_get::<SessionForm>("SessionForm").is_some());
    sess_form =
        SessionForm::update_session(server, &server.param_json().unwrap())
            .unwrap();
    assert_eq!(
        serde_json::to_string(&old_session).unwrap(),
        serde_json::to_string(&sess_form).unwrap()
    );
    assert!(server.session_get::<SessionForm>("SessionForm").is_none());
    println!("OK");

    print!("SessionForm update_session(), partial input ... ");
    sess_form = old_session.clone();
    assert!(sess_form.to_json_session(server).is_ok());
    server_stub
        .params
        .borrow_mut()
        .insert("rad".to_string(), "1".to_string());
    server_stub
        .params
        .borrow_mut()
        .insert("num".to_string(), "42".to_string());
    sess_form =
        SessionForm::update_session(server, &server.param_json().unwrap())
            .unwrap();
    assert_eq!(sess_form.rad.get(), Some(1));
    assert_eq!(sess_form.num.get(), Some(42));
    assert_eq!(sess_form.mul.get_mult(), old_session.mul.get_mult());
    assert!(server.session_get::<SessionForm>("SessionForm").is_none());
    println!("OK");

    print!("SessionForm update_session(), erronous input ... ");
    sess_form = old_session.clone();
    assert!(sess_form.to_json_session(server).is_ok());
    server_stub
        .params
        .borrow_mut()
        .insert("rad".to_string(), "17".to_string());
    server_stub
        .params
        .borrow_mut()
        .insert("num".to_string(), "this is not a number".to_string());
    match SessionForm::update_session(server, &server.param_json().unwrap()) {
        Ok(_) => panic!("should return error"),
        Err(sf) => {
            assert_eq!(
                sf.rad.error_iter().collect::<Vec<_>>(),
                vec!["update", r#""17""#],
            );
            let num_errors = sf.num.error_iter().collect::<Vec<_>>();
            assert_eq!(num_errors.len(), 3);
            assert_eq!(num_errors[0], "update");
            assert_eq!(num_errors[1], "this is not a number");
            assert_eq!(num_errors[2], "invalid digit found in string");
            assert_eq!(
                sf.error_iter().collect::<Vec<_>>(),
                vec![
                    "update",
                    &serde_json::to_string(&json!({
                        "num": "this is not a number",
                        "rad": "17",
                    }))
                    .unwrap(),
                ],
            );
        }
    }
    println!("OK");

    print!("SessionForm merge_model_error() ... ");
    let mut sf =
        SessionForm::update_session(server, &server.param_json().unwrap())
            .err()
            .unwrap();
    let error = cannot_save!(
        "SomeModel": "breaks-business-rules",
        "mul": ["mul-error-1", "mul-error-2"],
        "num_alias": ["required"],
    );
    let old_vals = vec![
        serde_json::to_value(sf.mul.get_mult()).unwrap(),
        serde_json::to_value(sf.num.get_mult()).unwrap(),
        serde_json::to_value(sf.rad.get_mult()).unwrap(),
    ];
    sf.merge_error(&error, &[("num_alias", "num")]);
    assert_eq!(
        vec![
            serde_json::to_value(sf.mul.get_mult()).unwrap(),
            serde_json::to_value(sf.num.get_mult()).unwrap(),
            serde_json::to_value(sf.rad.get_mult()).unwrap(),
        ],
        old_vals,
    );
    assert_eq!(
        sf.mul.error_iter().collect::<Vec<_>>(),
        vec![
            "error--CannotSave--SomeModel--mul--mul-error-1",
            "error--CannotSave--SomeModel--mul--mul-error-2",
        ],
    );
    assert_eq!(
        sf.num.error_iter().collect::<Vec<_>>(),
        vec![
            "update",
            "this is not a number",
            "invalid digit found in string",
            "error--CannotSave--SomeModel--num_alias--required",
        ],
    );
    assert_eq!(
        sf.rad.error_iter().collect::<Vec<_>>(),
        vec!["update", r#""17""#],
    );
    println!("OK");
}
