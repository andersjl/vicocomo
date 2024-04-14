//! Implement `vicocomo::TemplEng` by way of the `handlebars` crate.

use handlebars;
use std::sync::Mutex;

pub struct HbTemplEng(Mutex<handlebars::Handlebars<'static>>);

impl HbTemplEng {
    pub fn new(templ_dir: Option<&str>) -> Self {
        let mut hb = handlebars::Handlebars::new();
        hb.register_templates_directory(
            ".hbs",
            templ_dir.unwrap_or("templates"),
        )
        .unwrap();
        Self(Mutex::new(hb))
    }
}

impl vicocomo::TemplEng for HbTemplEng {
    fn initialized(&self) -> bool {
        !self.0.lock().unwrap().get_templates().is_empty()
    }

    fn register_templ_dir(
        &self,
        path: &str,
        ext: &str,
    ) -> Result<(), vicocomo::Error> {
        let mut eng = self.0.lock().unwrap();
        eng.clear_templates();
        eng.register_templates_directory(&(String::from(".") + ext), path)
            .map_err(|e| vicocomo::Error::render(e))
    }

    fn render(
        &self,
        tmpl: &str,
        jval: &serde_json::value::Value,
    ) -> Result<String, vicocomo::Error> {
        self.0
            .lock()
            .unwrap()
            .render(tmpl, jval)
            .map_err(|e| vicocomo::Error::render(e.to_string().as_str()))
    }
}
