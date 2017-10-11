use num;

/// This trait represents the operations a histogram must be able to perform on the underlying
/// counter type. The `ToPrimitive` trait is needed to perform floating point operations on the
/// counts (usually for quantiles). The `FromPrimitive` to convert back into an integer count.
/// Partial ordering is used for threshholding, also usually in the context of quantiles.
pub trait Counter
    : num::Num + num::ToPrimitive + num::FromPrimitive + num::Saturating + num::CheckedSub
    + num::CheckedAdd + Copy + PartialOrd<Self> {

    /// Counter as a f64.
    fn as_f64(&self) -> f64;
    /// Counter as a u64.
    fn as_u64(&self) -> u64;
}

impl Counter for u8 {
    #[inline]
    fn as_f64(&self) -> f64 { *self as f64 }
    #[inline]
        // Already checked that high >= 2*low
    fn as_u64(&self) -> u64 { *self as u64 }
}

impl Counter for u16 {
    #[inline]
    fn as_f64(&self) -> f64 { *self as f64 }
    #[inline]
    fn as_u64(&self) -> u64 { *self as u64 }
}

impl Counter for u32 {
    #[inline]
    fn as_f64(&self) -> f64 { *self as f64 }
    #[inline]
    fn as_u64(&self) -> u64 { *self as u64 }
}

impl Counter for u64 {
    #[inline]
    fn as_f64(&self) -> f64 { *self as f64 }
    #[inline]
    fn as_u64(&self) -> u64 { *self }
}
