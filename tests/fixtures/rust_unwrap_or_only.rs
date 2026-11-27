//! Fixture: `unwrap_or`-family recovery only (pretender-jg9y gap 2).
//!
//! `.unwrap_or*` recovers from None instead of panicking, so none of these
//! lines may count as an unwrap site.

fn picks(a: Option<u32>, b: Option<u32>, c: Option<u32>) -> u32 {
    let _x = a.unwrap_or(0);
    let _y = b.unwrap_or_else(fallback);
    let _z = c.unwrap_or_default();
    0
}

fn fallback() -> u32 {
    0
}
