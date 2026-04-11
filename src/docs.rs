//! Documented type system.
pub const ROOT_ATTR: &str = "root";
pub const ROWSET_ATTR: &str = "rowset";
pub const NAME_ATTR: &str = "name";
pub const AXIS_ATTR: &str = "axis";
pub const INCREMENT_BINDING_PREFIX: &str = "__rowview_increment_";
pub const ROWS_SUFFIX: &str = "Rows";

#[derive(Clone, Copy, Debug, PartialEq, Eq, strum::AsRefStr)]
#[strum(serialize_all = "snake_case")]
pub enum FieldKind {
    Agg,
    /// Repeated context
    Copy,
    FromAxis,
    FromIndex,
    Join,
    Select,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, strum::AsRefStr)]
#[strum(serialize_all = "snake_case")]
pub enum FieldMode {
    Direct,
    Increment,
}
