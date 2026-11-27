//! Fixture: six `.expect(...)` sites inside a `#[cfg(test)]`-gated module
//! (pretender-jg9y gap 2).
//!
//! With role=test these gated expects are excluded from the `unwraps`
//! count; the app-role control (same file at a repo root) keeps counting
//! them.

#[cfg(test)]
mod tests {
    fn inside_gate_a() {
        let value: Result<u8, ()> = Ok(1);
        let _ = value.expect("inside a");
    }

    fn inside_gate_b() {
        let value: Result<u8, ()> = Ok(1);
        let _ = value.expect("inside b");
    }

    fn inside_gate_c() {
        let value: Result<u8, ()> = Ok(1);
        let _ = value.expect("inside c");
    }

    fn inside_gate_d() {
        let value: Result<u8, ()> = Ok(1);
        let _ = value.expect("inside d");
    }

    fn inside_gate_e() {
        let value: Result<u8, ()> = Ok(1);
        let _ = value.expect("inside e");
    }

    fn inside_gate_f() {
        let value: Result<u8, ()> = Ok(1);
        let _ = value.expect("inside f");
    }
}
