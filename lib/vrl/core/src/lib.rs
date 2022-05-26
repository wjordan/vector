#![deny(clippy::all)]
#![deny(unreachable_pub)]
#![deny(unused_allocation)]
#![deny(unused_extern_crates)]
#![deny(unused_assignments)]
#![deny(unused_comparisons)]

mod expression;
mod r#macro;
mod secrets;
mod target;

pub use expression::{ExpressionError, Resolved};
pub use secrets::Secrets;
pub use target::{MetadataTarget, Target, TargetValue, TargetValueRef};
pub use value::Value;
