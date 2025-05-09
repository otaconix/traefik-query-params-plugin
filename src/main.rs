use indexmap::IndexMap;
use lazy_static::lazy_static;
use serde::Deserialize;
use serde::Serialize;
use traefik_wasm_api as traefik;

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

fn position_to_index<T>(position: i8, vec: &[T], for_insertion: bool) -> usize {
    if position < 0 {
        vec.len()
            .checked_sub(position.unsigned_abs() as usize)
            .map(|i| if for_insertion { i + 1 } else { i })
            .unwrap_or(0)
    } else {
        (position as usize).min(vec.len())
    }
}

#[derive(Serialize, Deserialize)]
struct AddOperation {
    position: i8,
    value: Option<String>,
}

impl AddOperation {
    fn apply(&self, name: &str, query: &mut Vec<(String, String)>) {
        let index = position_to_index(self.position, query, true);

        query.insert(
            index,
            (
                name.to_string(),
                self.value.clone().unwrap_or("".to_string()),
            ),
        )
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum RemoveOperation {
    All,
    Position(i8),
}

impl RemoveOperation {
    fn apply(&self, name: &str, query: &mut Vec<(String, String)>) {
        match self {
            RemoveOperation::All => query.retain(|(param_name, _)| param_name != name),
            RemoveOperation::Position(position) => {
                let matching_param_indices = query
                    .iter()
                    .enumerate()
                    .filter(|(_, (key, _))| key == name)
                    .map(|(index, _)| index)
                    .collect::<Vec<_>>();
                if !matching_param_indices.is_empty() {
                    let index_to_remove =
                        position_to_index(*position, &matching_param_indices, false);
                    query.remove(index_to_remove);
                }
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
enum QueryParamOperation {
    #[serde(rename = "add")]
    Add(AddOperation),
    #[serde(rename = "remove")]
    Remove(RemoveOperation),
}

#[derive(Serialize, Deserialize)]
struct Config {
    #[serde(flatten)]
    params: IndexMap<String, QueryParamOperation>,
}

lazy_static! {
    static ref CONFIG: Config = get_config();
}

fn get_config() -> Config {
    match serde_json::from_slice(&traefik::get_conf()) {
        Ok(config) => config,
        Err(e) => {
            traefik::send_log(traefik::WARN, &format!("Invalid configuration: {e}"));

            Config {
                params: IndexMap::new(),
            }
        }
    }
}

#[unsafe(export_name = "handle_request")]
fn http_request() -> i64 {
    let request_uri = traefik::get_request_uri();
    let mut url = url::Url::parse(&format!("http://example.invalid{request_uri}")).unwrap();
    let mut original_query = url
        .query_pairs()
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect::<Vec<_>>();

    {
        let mut query = url.query_pairs_mut();
        query.clear();
        for (key, operation) in CONFIG.params.iter() {
            match operation {
                QueryParamOperation::Add(add_operation) => {
                    add_operation.apply(key, &mut original_query)
                }
                QueryParamOperation::Remove(remove_operation) => {
                    remove_operation.apply(key, &mut original_query)
                }
            }
        }
        query.extend_pairs(original_query.iter());
    }

    traefik::set_request_uri(&format!(
        "{}{}",
        url.path(),
        url.query()
            .map(|query| format!("?{query}"))
            .unwrap_or("".to_string())
    ));

    1
}

#[unsafe(export_name = "handle_response")]
fn http_response(_req_ctx: i32, _is_error: i32) {}

fn main() {}

#[cfg(test)]
mod tests {
    use super::*;

    fn pair<Value: ToString + Sized>(key: &str, value: Value) -> (String, String) {
        (key.to_string(), value.to_string())
    }

    #[test]
    fn serialization() {
        serde_json::to_writer_pretty(
            std::io::stdout(),
            &Config {
                params: IndexMap::from([
                    (
                        "test-param".to_string(),
                        QueryParamOperation::Add(AddOperation {
                            position: 0,
                            value: Some("Hello world!".to_string()),
                        }),
                    ),
                    (
                        "remove-all".to_string(),
                        QueryParamOperation::Remove(RemoveOperation::All),
                    ),
                    (
                        "remove-one".to_string(),
                        QueryParamOperation::Remove(RemoveOperation::Position(-2)),
                    ),
                ]),
            },
        )
        .unwrap();
    }

    #[test]
    fn add_negative() {
        let mut query = [pair("one", 1)].to_vec();

        AddOperation {
            position: -1,
            value: Some("2".to_string()),
        }
        .apply("two", &mut query);

        assert_eq!(
            &query,
            &[
                ("one".to_string(), "1".to_string()),
                ("two".to_string(), "2".to_string())
            ]
        );
    }

    #[test]
    fn add_negative_overflow() {
        let mut query = [pair("one", 1)].to_vec();

        AddOperation {
            position: -5,
            value: Some("2".to_string()),
        }
        .apply("two", &mut query);

        assert_eq!(&query, &[pair("two", 2), pair("one", 1),]);
    }

    #[test]
    fn remove_all() {
        let mut query = [pair("remove-me", 1), pair("keep", 2), pair("remove-me", 3)].to_vec();

        RemoveOperation::All.apply("remove-me", &mut query);

        assert_eq!(&query, &[pair("keep", 2)]);
    }

    #[test]
    fn remove_positive() {
        let mut query = [
            pair("key", "kept1"),
            pair("key", "removed"),
            pair("key", "kept2"),
        ]
        .to_vec();

        RemoveOperation::Position(1).apply("key", &mut query);

        assert_eq!(&query, &[pair("key", "kept1"), pair("key", "kept2")]);
    }

    #[test]
    fn remove_negative() {
        let mut query = [
            pair("key", "kept1"),
            pair("key", "removed"),
            pair("key", "kept2"),
        ]
        .to_vec();

        RemoveOperation::Position(-2).apply("key", &mut query);

        assert_eq!(&query, &[pair("key", "kept1"), pair("key", "kept2")]);
    }
}
