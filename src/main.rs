mod add;
mod remove;
mod util;

use add::AddOperation;
use indexmap::IndexMap;
use lazy_static::lazy_static;
use remove::RemoveOperation;
use serde::Deserialize;
use traefik_wasm_api as traefik;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum QueryParamOperation {
    Add(AddOperation),
    Remove(RemoveOperation),
}

#[derive(Deserialize, Debug)]
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
    use serde_json::json;

    use super::*;
    use add::AddOperation;
    use remove::RemoveOperation;

    fn pair<Value: ToString + Sized>(key: &str, value: Value) -> (String, String) {
        (key.to_string(), value.to_string())
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

        RemoveOperation::all().apply("remove-me", &mut query);

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

        RemoveOperation::position(1).apply("key", &mut query);

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

        RemoveOperation::position(-2).apply("key", &mut query);

        assert_eq!(&query, &[pair("key", "kept1"), pair("key", "kept2")]);
    }

    #[test]
    fn deserialize_add_position() {
        let _add_operation_from_string: AddOperation =
            serde_json::from_value(json!({"position": "-1"})).unwrap();
        let _add_operation_from_number: AddOperation =
            serde_json::from_value(json!({"position": -1})).unwrap();
    }

    #[test]
    fn deserialize_remove() {
        let remove_operation_from_string: RemoveOperation =
            serde_json::from_value(json!({"position": "-1"})).unwrap();
        assert_eq!(remove_operation_from_string, RemoveOperation::position(-1));
        let remove_operation_from_number: RemoveOperation =
            serde_json::from_value(json!({"position": -1})).unwrap();
        assert_eq!(remove_operation_from_number, RemoveOperation::position(-1));
        let remove_operation_all: RemoveOperation = serde_json::from_value(json!({})).unwrap();
        assert_eq!(remove_operation_all, RemoveOperation::all());
        let remove_operation_all_explicit_null: RemoveOperation =
            serde_json::from_value(json!({"position": null})).unwrap();
        assert_eq!(remove_operation_all_explicit_null, RemoveOperation::all());
        let remove_operation_all_regexp: RemoveOperation =
            serde_json::from_value(json!({"regexp": "hello.*"})).unwrap();
        assert_eq!(
            remove_operation_all_regexp,
            RemoveOperation::all().matching_regexp("hello.*").unwrap()
        );
        let remove_operation_position_regexp: RemoveOperation = serde_json::from_value(json!({
            "position": "-2",
            "regexp": "hello.*"
        }))
        .unwrap();
        assert_eq!(
            remove_operation_position_regexp,
            RemoveOperation::position(-2)
                .matching_regexp("hello.*")
                .unwrap()
        );
    }
}
