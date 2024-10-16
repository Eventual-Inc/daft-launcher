use serde::Deserialize;

use super::ProcessableOption;

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
struct Config {
    flag: ProcessableOption<bool>,
}

#[test]
fn test_deser_with_no_value() {
    let result = toml::from_str("");
    assert_eq!(
        result,
        Ok(Config {
            flag: ProcessableOption::new(None),
        })
    );
}

#[test]
fn test_deser_with_value() {
    let result = toml::from_str("flag = true");
    assert_eq!(
        result,
        Ok(Config {
            flag: ProcessableOption::new(Some(true)),
        })
    );
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
struct ConfigWithNestedOption {
    flag: ProcessableOption<Option<bool>>,
}

#[test]
fn test_deser_with_nested_option_with_no_value() {
    let result = toml::from_str("");
    assert_eq!(
        result,
        Ok(ConfigWithNestedOption {
            flag: ProcessableOption::new(None)
        })
    );
}

#[test]
fn test_deser_with_nested_option_with_value() {
    let result = toml::from_str("flag = true");
    assert_eq!(
        result,
        Ok(ConfigWithNestedOption {
            flag: ProcessableOption::new(Some(Some(true)))
        })
    );
}

#[test]
fn test() {
    let x = ProcessableOption::from(Some(0u8));
}
