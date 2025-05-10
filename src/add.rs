use serde::Deserialize;
use serde_aux::field_attributes::deserialize_number_from_string;

use crate::util::position_to_index;

#[derive(Debug, Deserialize)]
pub struct AddOperation {
    #[serde(
        deserialize_with = "deserialize_number_from_string",
        default = "AddOperation::default_position"
    )]
    pub(crate) position: i8,
    pub(crate) value: Option<String>,
}

impl AddOperation {
    pub fn apply(&self, name: &str, query: &mut Vec<(String, String)>) {
        let index = position_to_index(self.position, query, true);

        query.insert(
            index,
            (
                name.to_string(),
                self.value.clone().unwrap_or("".to_string()),
            ),
        )
    }

    fn default_position() -> i8 {
        -1
    }
}
