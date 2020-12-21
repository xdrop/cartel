use chrono::Local;
use std::convert::TryFrom;

/// Returns the number of non-leap seconds since
/// January 1, 1970 0:00:00 UTC (aka "UNIX timestamp").
///
/// To be used when time is not expected to be before 1970. The epoch unlike
/// `chrono::Local` is casted to u64 because of this and will panic if an
/// invalid date is passed.
pub fn epoch_now() -> u64 {
    u64::try_from(Local::now().timestamp())
        .expect("Got date before 1970, this is unsupported by epoch_now")
}
