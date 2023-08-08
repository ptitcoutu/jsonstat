use itertools::Itertools;
use std::fmt::Error;
use std::io::Read;
use std::result::IntoIter;

use crate::json_stat_extractor::JsonStat::{ArrayStat, ObjStat, ValStat};
use serde::{Deserialize, Serialize};
use serde_json::Value::{Array, Object, String};
use serde_json::{from_reader, Value};

const DOUBLE_QUOTES_SIZE: usize = 2;
const CURLY_BRACKETS_SIZE: usize = 2;
const SEMI_COLON_SIZE: usize = 1;

pub fn extract_stat_from_json<R>(json_content_reader: R) -> JsonStat
where
    R: Read,
{
    let json_value_stream: IntoIter<Value> = from_reader(json_content_reader).into_iter();
    return extract_stat_from_json_iter(json_value_stream);
}

#[derive(Serialize, Deserialize)]
pub enum JsonStat {
    ValStat(JsonValStat),
    ObjStat(JsonObjStat),
    ArrayStat(JsonArrayStat),
}

pub fn json_stat_size(json_stat: &JsonStat) -> usize {
    return match json_stat {
        ValStat(vs) => vs.size,
        ObjStat(vs) => vs.size,
        ArrayStat(vs) => vs.size,
    };
}

#[derive(Serialize, Deserialize)]
pub struct JsonAttrStat {
    name: std::string::String,
    size: usize,
    count: usize,
    max_size: usize,
    min_size: usize,
}

#[derive(Serialize, Deserialize)]
pub struct JsonValStat {
    size: usize,
    max_size: usize,
    min_size: usize,
}

#[derive(Serialize, Deserialize)]
pub struct JsonObjStat {
    size: usize,
    count: usize,
    max_size: usize,
    min_size: usize,
    attributes: Vec<JsonAttrStat>,
}

#[derive(Serialize, Deserialize)]
pub struct JsonArrayStat {
    size: usize,
    count: usize,
    max_size: usize,
    min_size: usize,
    attributes: Vec<JsonAttrStat>,
}

pub fn extract_stat_from_json_iter(json_value_stream: IntoIter<Value>) -> JsonStat {
    let stats = json_value_stream
        .map(|json_value| {
            let v_size = match json_value {
                Value::Null => ValStat(JsonValStat {
                    size: 4,
                    max_size: 4,
                    min_size: 4,
                }),
                String(txt) => ValStat(JsonValStat {
                    size: txt.len() + DOUBLE_QUOTES_SIZE,
                    max_size: txt.len() + DOUBLE_QUOTES_SIZE,
                    min_size: txt.len() + DOUBLE_QUOTES_SIZE,
                }),
                Object(vals) => {
                    let attr_stats: Vec<JsonAttrStat> = vals
                        .into_iter()
                        .map(|attr| {
                            let result_value: Result<Value, Error> = Ok(attr.1);
                            let json_iter: IntoIter<Value> = result_value.into_iter();
                            let val_stat = extract_stat_from_json_iter(json_iter);
                            let val_size = json_stat_size(&val_stat);
                            return JsonAttrStat {
                                name: attr.0,
                                size: val_size,
                                count: 1,
                                max_size: val_size,
                                min_size: val_size,
                            };
                        })
                        .collect();
                    let total_size_inside_curly_brackets: usize = attr_stats
                        .iter()
                        .map(|attr_stat| {
                            let double_quotes_size_for_name = DOUBLE_QUOTES_SIZE;
                            return attr_stat.size
                                + attr_stat.name.len()
                                + double_quotes_size_for_name
                                + SEMI_COLON_SIZE;
                        })
                        .sum();
                    let total_size: usize = total_size_inside_curly_brackets + CURLY_BRACKETS_SIZE;
                    return ObjStat(JsonObjStat {
                        size: total_size,
                        count: 1,
                        max_size: total_size,
                        min_size: total_size,
                        attributes: attr_stats,
                    });
                }
                Array(vals) => {
                    let item_stats: Vec<JsonStat> = vals
                        .into_iter()
                        .map(|attr| {
                            let result_value: Result<Value, Error> = Ok(attr);
                            let json_iter: IntoIter<Value> = result_value.into_iter();
                            return extract_stat_from_json_iter(json_iter);
                        })
                        .collect();
                    let total_count = item_stats.len();
                    let size_of_comma = total_count - 1;
                    let size_of_brackets = 2;
                    let total_size = if total_count > 0 {
                        let sizes_sum: usize = item_stats
                            .iter()
                            .map(|json_stat| json_stat_size(json_stat))
                            .sum();
                        sizes_sum + size_of_comma + size_of_brackets
                    } else {
                        0
                    };
                    let min_size = if total_count > 0 {
                        let sizes_min: Option<usize> = item_stats
                            .iter()
                            .map(|json_stat| json_stat_size(json_stat))
                            .min();
                        sizes_min.unwrap()
                    } else {
                        0
                    };
                    let max_size = if total_count > 0 {
                        let sizes_max: Option<usize> = item_stats
                            .iter()
                            .map(|json_stat| json_stat_size(json_stat))
                            .max();
                        sizes_max.unwrap()
                    } else {
                        0
                    };
                    let attr_stats: Vec<JsonAttrStat> = item_stats
                        .into_iter()
                        .flat_map(|json_stat| {
                            let attrs = match json_stat {
                                ObjStat(JsonObjStat { attributes, .. }) => attributes,
                                _ => vec![],
                            };
                            return attrs;
                        })
                        .into_group_map_by(|json_attr_stat| (json_attr_stat.name.clone()))
                        .into_iter()
                        .map(|attr_stat_by_name| {
                            let attr_name = attr_stat_by_name.0;
                            let attr_stats = attr_stat_by_name.1;
                            let attr_sizes_and_counts: Vec<Vec<usize>> = attr_stats
                                .iter()
                                .map(|stat| {
                                    vec![stat.size, stat.count, stat.min_size, stat.max_size]
                                })
                                .collect();
                            let attr_sizes =
                                attr_sizes_and_counts.clone().into_iter().map(|it| it[0]);
                            let attr_counts =
                                attr_sizes_and_counts.clone().into_iter().map(|it| it[1]);
                            let attr_count = attr_counts.sum();
                            let attr_total_sizes: usize = attr_sizes.sum();
                            let attr_avg_size = attr_total_sizes
                                / attr_sizes_and_counts.clone().into_iter().count();
                            let attr_min_sizes =
                                attr_sizes_and_counts.clone().into_iter().map(|it| it[2]);
                            let attr_min_size = attr_min_sizes.min().unwrap_or(0);
                            let attr_max_sizes =
                                attr_sizes_and_counts.clone().into_iter().map(|it| it[3]);
                            let attr_max_size = attr_max_sizes.max().unwrap_or(0);
                            return JsonAttrStat {
                                name: attr_name,
                                size: attr_avg_size,
                                count: attr_count,
                                max_size: attr_max_size,
                                min_size: attr_min_size,
                            };
                        })
                        .collect();
                    return ArrayStat(JsonArrayStat {
                        size: total_size,
                        count: total_count,
                        max_size,
                        min_size,
                        attributes: attr_stats,
                    });
                }
                Value::Bool(val) => ValStat(JsonValStat {
                    size: val.to_string().len(),
                    max_size: val.to_string().len(),
                    min_size: val.to_string().len(),
                }),
                Value::Number(val) => ValStat(JsonValStat {
                    size: val.to_string().len(),
                    max_size: val.to_string().len(),
                    min_size: val.to_string().len(),
                }),
            };
            return v_size;
        })
        .nth(0)
        .unwrap();
    return stats;
}

#[cfg(test)]
mod tests {
    use std::fmt::Error;
    use std::result::IntoIter;

    use serde_json::{json, Value};

    use JsonStat::ValStat;

    use crate::json_stat_extractor::JsonStat::{ArrayStat, ObjStat};
    use crate::json_stat_extractor::{
        extract_stat_from_json_iter, JsonArrayStat, JsonObjStat, JsonStat, JsonValStat,
    };

    #[test]
    fn it_should_provide_size_of_json_value() {
        let result_value: Result<Value, Error> = Ok(json!("test"));
        let json_iter: IntoIter<Value> = result_value.into_iter();
        let result = extract_stat_from_json_iter(json_iter);
        match result {
            ValStat(JsonValStat {
                size,
                max_size,
                min_size,
            }) => {
                assert_eq!(size, 6);
                assert_eq!(max_size, 6);
                assert_eq!(min_size, 6);
            }
            _ => {
                assert!(false);
            }
        }
    }

    #[test]
    fn it_should_provide_size_of_json_null() {
        let result_value: Result<Value, Error> = Ok(json!(null));
        let json_iter: IntoIter<Value> = result_value.into_iter();
        let result = extract_stat_from_json_iter(json_iter);
        match result {
            ValStat(JsonValStat {
                size,
                max_size,
                min_size,
            }) => {
                assert_eq!(size, 4);
                assert_eq!(max_size, 4);
                assert_eq!(min_size, 4);
            }
            _ => {
                assert!(false);
            }
        }
    }

    #[test]
    fn it_should_provide_size_of_json_true() {
        let result_value: Result<Value, Error> = Ok(json!(true));
        let json_iter: IntoIter<Value> = result_value.into_iter();
        let result = extract_stat_from_json_iter(json_iter);
        match result {
            ValStat(JsonValStat {
                size,
                max_size,
                min_size,
            }) => {
                assert_eq!(size, 4);
                assert_eq!(max_size, 4);
                assert_eq!(min_size, 4);
            }
            _ => {
                assert!(false);
            }
        }
    }

    #[test]
    fn it_should_provide_size_of_json_object() {
        let result_value: Result<Value, Error> = Ok(json!({"test":"test"}));
        let json_iter: IntoIter<Value> = result_value.into_iter();
        let result = extract_stat_from_json_iter(json_iter);
        match result {
            ObjStat(JsonObjStat {
                size,
                count,
                max_size,
                min_size,
                attributes,
            }) => {
                assert_eq!(size, 15);
                assert_eq!(count, 1);
                assert_eq!(max_size, 15);
                assert_eq!(min_size, 15);
                assert_eq!(attributes.len(), 1);
                let attr_stat = attributes.first().unwrap();
                assert_eq!(attr_stat.name, "test");
                assert_eq!(attr_stat.count, 1);
                assert_eq!(attr_stat.size, 6);
                assert_eq!(attr_stat.max_size, 6);
                assert_eq!(attr_stat.min_size, 6);
            }
            _ => {
                assert!(false);
            }
        }
    }

    #[test]
    fn it_should_provide_size_of_json_array() {
        let result_value: Result<Value, Error> = Ok(json!(["test", "test0123456789"]));
        let json_iter: IntoIter<Value> = result_value.into_iter();
        let result = extract_stat_from_json_iter(json_iter);
        match result {
            ArrayStat(JsonArrayStat {
                size,
                count,
                max_size,
                min_size,
                attributes,
            }) => {
                assert_eq!(min_size, 6);
                assert_eq!(max_size, 16);
                assert_eq!(size, 25);
                assert_eq!(count, 2);
                assert_eq!(attributes.len(), 0);
            }
            _ => {
                assert!(false);
            }
        }
    }

    #[test]
    fn it_should_provide_size_of_json_array_of_objects() {
        let result_value: Result<Value, Error> =
            Ok(json!([{"test":"test"}, {"test":"test3", "b": true}]));
        let json_iter: IntoIter<Value> = result_value.into_iter();
        let result = extract_stat_from_json_iter(json_iter);
        match result {
            ArrayStat(JsonArrayStat {
                size,
                count,
                max_size,
                min_size,
                attributes,
            }) => {
                assert_eq!(min_size, 15);
                assert_eq!(max_size, 24);
                assert_eq!(size, 42);
                assert_eq!(count, 2);
                assert_eq!(attributes.len(), 2);
                let test_attribute = attributes.get(1).unwrap();
                assert_eq!(test_attribute.name, "test");
                assert_eq!(test_attribute.min_size, 6);
                assert_eq!(test_attribute.max_size, 7);
                assert_eq!(test_attribute.size, 6);
                assert_eq!(test_attribute.count, 2);
                let b_attribute = attributes.get(0).unwrap();
                assert_eq!(b_attribute.name, "b");
            }
            _ => {
                assert!(false);
            }
        }
    }
}
