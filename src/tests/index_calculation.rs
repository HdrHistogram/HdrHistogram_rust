use super::super::{CreationError, Histogram};
use tests::helpers::histo64;

#[test]
fn unit_magnitude_0_index_calculations() {
    let h = histo64(1_u64, 1_u64 << 32, 3);
    assert_eq!(2048, h.sub_bucket_count);
    assert_eq!(0, h.unit_magnitude);
    // sub_bucket_count = 2^11, so 2^11 << 22 is > the max of 2^32 for 23 buckets total
    assert_eq!(23, h.bucket_count);

    // first half of first bucket
    assert_eq!(0, h.bucket_for(3));
    assert_eq!(3, h.sub_bucket_for(3, 0));

    // second half of first bucket
    assert_eq!(0, h.bucket_for(1024 + 3));
    assert_eq!(1024 + 3, h.sub_bucket_for(1024 + 3, 0));

    // second bucket (top half)
    assert_eq!(1, h.bucket_for(2048 + 3 * 2));
    // counting by 2s, starting at halfway through the bucket
    assert_eq!(1024 + 3, h.sub_bucket_for(2048 + 3 * 2, 1));

    // third bucket (top half)
    assert_eq!(2, h.bucket_for((2048 << 1) + 3 * 4));
    // counting by 4s, starting at halfway through the bucket
    assert_eq!(1024 + 3, h.sub_bucket_for((2048 << 1) + 3 * 4, 2));

    // past last bucket -- not near u64::max_value(), so should still calculate ok.
    assert_eq!(23, h.bucket_for((2048_u64 << 22) + 3 * (1 << 23)));
    assert_eq!(1024 + 3, h.sub_bucket_for((2048_u64 << 22) + 3 * (1 << 23), 23));
}

#[test]
fn unit_magnitude_4_index_calculations() {
    let h = histo64(1_u64 << 12, 1_u64 << 32, 3);
    assert_eq!(2048, h.sub_bucket_count);
    assert_eq!(12, h.unit_magnitude);
    // sub_bucket_count = 2^11. With unit magnitude shift, it's 2^23. 2^23 << 10 is > the max of
    // 2^32 for 11 buckets total
    assert_eq!(11, h.bucket_count);
    let unit = 1_u64 << 12;

    // below lowest value
    assert_eq!(0, h.bucket_for(3));
    assert_eq!(0, h.sub_bucket_for(3, 0));

    // first half of first bucket
    assert_eq!(0, h.bucket_for(3 * unit));
    assert_eq!(3, h.sub_bucket_for(3 * unit, 0));

    // second half of first bucket
    // sub_bucket_half_count's worth of units, plus 3 more
    assert_eq!(0, h.bucket_for(unit * (1024 + 3)));
    assert_eq!(1024 + 3, h.sub_bucket_for(unit * (1024 + 3), 0));

    // second bucket (top half), bucket scale = unit << 1.
    // Middle of bucket is (sub_bucket_half_count = 2^10) of bucket scale, = unit << 11.
    // Add on 3 of bucket scale.
    assert_eq!(1, h.bucket_for((unit << 11) + 3 * (unit << 1)));
    assert_eq!(1024 + 3, h.sub_bucket_for((unit << 11) + 3 * (unit << 1), 1));

    // third bucket (top half), bucket scale = unit << 2.
    // Middle of bucket is (sub_bucket_half_count = 2^10) of bucket scale, = unit << 12.
    // Add on 3 of bucket scale.
    assert_eq!(2, h.bucket_for((unit << 12) + 3 * (unit << 2)));
    assert_eq!(1024 + 3, h.sub_bucket_for((unit << 12) + 3 * (unit << 2), 2));

    // past last bucket -- not near u64::max_value(), so should still calculate ok.
    assert_eq!(11, h.bucket_for((unit << 21) + 3 * (unit << 11)));
    assert_eq!(1024 + 3, h.sub_bucket_for((unit << 21) + 3 * (unit << 11), 11));
}

#[test]
fn unit_magnitude_52_sub_bucket_magnitude_11_index_calculations() {
    // maximum unit magnitude for this precision
    let h = histo64(1_u64 << 52, u64::max_value(), 3);
    assert_eq!(2048, h.sub_bucket_count);
    assert_eq!(52, h.unit_magnitude);
    // sub_bucket_count = 2^11. With unit magnitude shift, it's 2^63. 1 more bucket to (almost)
    // reach 2^64.
    assert_eq!(2, h.bucket_count);
    assert_eq!(1, h.leading_zero_count_base);
    let unit = 1_u64 << 52;

    // below lowest value
    assert_eq!(0, h.bucket_for(3));
    assert_eq!(0, h.sub_bucket_for(3, 0));

    // first half of first bucket
    assert_eq!(0, h.bucket_for(3 * unit));
    assert_eq!(3, h.sub_bucket_for(3 * unit, 0));

    // second half of first bucket
    // sub_bucket_half_count's worth of units, plus 3 more
    assert_eq!(0, h.bucket_for(unit * (1024 + 3)));
    assert_eq!(1024 + 3, h.sub_bucket_for(unit * (1024 + 3), 0));

    // end of second half
    assert_eq!(0, h.bucket_for(unit * 1024 + 1023 * unit));
    assert_eq!(1024 + 1023, h.sub_bucket_for(unit * 1024 + 1023 * unit, 0));

    // second bucket (top half), bucket scale = unit << 1.
    // Middle of bucket is (sub_bucket_half_count = 2^10) of bucket scale, = unit << 11.
    // Add on 3 of bucket scale.
    assert_eq!(1, h.bucket_for((unit << 11) + 3 * (unit << 1)));
    assert_eq!(1024 + 3, h.sub_bucket_for((unit << 11) + 3 * (unit << 1), 1));

    // upper half of second bucket, last slot
    assert_eq!(1, h.bucket_for(u64::max_value()));
    assert_eq!(1024 + 1023, h.sub_bucket_for(u64::max_value(), 1));
}

#[test]
fn unit_magnitude_53_sub_bucket_magnitude_11_throws() {
    assert_eq!(CreationError::CannotRepresentSigFigBeyondLow,
        Histogram::<u64>::new_with_bounds(1_u64 << 53, 1_u64 << 63, 3).unwrap_err());
}

#[test]
fn unit_magnitude_55_sub_bucket_magnitude_8_ok() {
    let h = histo64(1_u64 << 55, 1_u64 << 63, 2);
    assert_eq!(256, h.sub_bucket_count);
    assert_eq!(55, h.unit_magnitude);
    // sub_bucket_count = 2^8. With unit magnitude shift, it's 2^63.
    assert_eq!(2, h.bucket_count);

    // below lowest value
    assert_eq!(0, h.bucket_for(3));
    assert_eq!(0, h.sub_bucket_for(3, 0));

    // upper half of second bucket, last slot
    assert_eq!(1, h.bucket_for(u64::max_value()));
    assert_eq!(128 + 127, h.sub_bucket_for(u64::max_value(), 1));
}

#[test]
fn unit_magnitude_62_sub_bucket_magnitude_1_ok() {
    let h = histo64(1_u64 << 62, 1_u64 << 63, 0);
    assert_eq!(2, h.sub_bucket_count);
    assert_eq!(62, h.unit_magnitude);
    // sub_bucket_count = 2^1. With unit magnitude shift, it's 2^63.
    assert_eq!(2, h.bucket_count);

    // below lowest value
    assert_eq!(0, h.bucket_for(3));
    assert_eq!(0, h.sub_bucket_for(3, 0));

    // upper half of second bucket, last slot
    assert_eq!(1, h.bucket_for(u64::max_value()));
    assert_eq!(1, h.sub_bucket_for(u64::max_value(), 1));
}
