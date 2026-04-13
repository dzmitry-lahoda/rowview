//! Documented type system.

pub const ROOT_ATTR: &str = "root";
pub const ROWSET_ATTR: &str = "rowset";
pub const SUPPORT_ATTR: &str = "support";
pub const BIND_ATTR: &str = "bind";
pub const NAME_ATTR: &str = "name";
pub const AXIS_ATTR: &str = "axis";
pub const KEY_ATTR: &str = "key";
pub const ITEM_ATTR: &str = "item";
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
    FromKey,
    Join,
    Select,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, strum::AsRefStr)]
#[strum(serialize_all = "snake_case")]
pub enum FieldMode {
    Direct,
    Increment,
}

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, strum::AsRefStr, strum::EnumString, strum::VariantNames,
)]
#[strum(serialize_all = "snake_case")]
pub enum JoinKey {
    Left,
    From,
    Must,
    Inner,
    Zip,
    Index,
    As,
    Alias,
    Option,
    On,
    Value,
    Select,
    By,
}

impl TryFrom<proc_macro2::Ident> for JoinKey {
    type Error = syn::Error;

    fn try_from(ident: proc_macro2::Ident) -> Result<Self, Self::Error> {
        ident
            .to_string()
            .parse()
            .map_err(|_| syn::Error::new(ident.span(), expected_join_key_message()))
    }
}

fn expected_join_key_message() -> String {
    format!(
        "expected {}",
        <JoinKey as strum::VariantNames>::VARIANTS
            .iter()
            .map(|key| format!("`{key}`"))
            .collect::<Vec<_>>()
            .join(", ")
    )
}
