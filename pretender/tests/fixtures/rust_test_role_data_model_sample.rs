// pretender: role = test
// Fixture: same 10-field shape as rust_data_model_sample.rs with an
// explicit test-role pragma, so `[thresholds.test]
// excessive_fields_max = 0` must silence the check end-to-end
// (add-data-model-checks §7.2).

pub struct CustomerManifest {
    pub full_name: String,
    pub email: String,
    pub home_city: String,
    pub postal_code: String,
    pub region_name: String,
    pub loyalty_tier: u32,
    pub visit_count: u32,
    pub lifetime_value: f64,
    pub signup_channel: String,
    pub preferred_locale: String,
}

impl CustomerManifest {
    pub fn greeting(&self) -> String {
        format!("hello, {}", self.full_name)
    }
}
