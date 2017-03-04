#[path = "tests.rs"]
#[cfg(test)]
mod tests;


/// Map signed numbers to unsigned: 0 to 0, -1 to 1, 1 to 2, -2 to 3, etc
#[allow(dead_code)] // TODO
fn zig_zag_encode(num: i64) -> u64 {
    // If num < 0, num >> 63 is all 1 and vice versa.
    ((num << 1) ^ (num >> 63)) as u64
}

#[allow(dead_code)] // TODO
fn zig_zag_decode(encoded: u64) -> i64 {
    ((encoded >> 1) as i64) ^ -((encoded & 1) as i64)
}
