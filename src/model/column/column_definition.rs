use serde::{Deserialize, Serialize};

use super::column_data_type::ColumnDataType;

/// Column definition used in DDL.
#[derive(Eq, PartialEq, Debug, Serialize, Deserialize, new)]
pub(crate) struct ColumnDefinition {
    column_data_type: ColumnDataType,
    // TODO column_constraints like DEFAULT
}

impl ColumnDefinition {
    pub(crate) fn column_data_type(&self) -> &ColumnDataType {
        &self.column_data_type
    }
}
