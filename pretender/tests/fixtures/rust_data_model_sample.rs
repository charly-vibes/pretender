// Fixture: data-model wiring (add-data-model-checks §7.1).
// The 10 fields trip `excessive_fields` at the default app limit (10);
// the `email: String` field trips `primitive_field` when
// `primitive_field_check` is enabled in pretender.toml.

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
    // Methods stay CodeUnits: they must not be counted as fields.
    pub fn greeting(&self) -> String {
        format!("hello, {}", self.full_name)
    }
}
