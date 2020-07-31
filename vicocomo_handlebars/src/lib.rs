//! Implement `vicocomo::TemplEng` by way of the `handlebars` crate.

use handlebars;

pub struct HbTemplEng(handlebars::Handlebars);

impl HbTemplEng {
    pub fn new(templ_dir: Option<&str>) -> Self {
        let mut hb = handlebars::Handlebars::new();
        hb.register_templates_directory(
            ".hbs",
            templ_dir.unwrap_or("templates"),
        )
        .unwrap();
        Self(hb)
    }
}

impl vicocomo::TemplEng for HbTemplEng {
    fn render(
        &self,
        tmpl: &str,
        data: &impl serde::Serialize,
    ) -> Result<String, vicocomo::Error> {
        self.0
            .render(tmpl, data)
            .map_err(|e| vicocomo::Error::render(e.to_string().as_str()))
    }
}
